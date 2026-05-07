/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, Metadata, create_dir_all, hard_link, metadata, remove_dir_all, remove_file, rename},
    io::{BufReader, ErrorKind as IOErrorKind, Read, Result as IOResult, Write, copy},
    os::unix::{fs::symlink, prelude::MetadataExt},
    path::Path,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use bincode::Options;
use dialoguer::console::Term;
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use signal_hook::iterator::Signals;
use thiserror::Error as ThisError;
use walkdir::WalkDir;
use zstd::{Decoder, Encoder};

use crate::{
    ErrorExt,
    ErrorKind,
    ErrorTrait,
    PathContext,
    Result,
    config::{ContainerCache, ContainerHandle, ContainerType::*},
    constants::{BAR_GREEN, BOLD, DATA_DIR, RESET, SIGNAL_LIST},
    eprintln_warn,
    impl_error,
    lock::{Lock, LockError},
    sync::{
        SyncError,
        transaction::aggregator::{BAR_CYAN_STYLE, BAR_GREEN_STYLE},
    },
    utils::bytebuffer::ByteBuffer,
};

const VERSION: u32 = 3;
const MAGIC_NUMBER: u32 = 408948530;
const BYTE_LIMIT: u64 = 134217728;

#[derive(Serialize, Deserialize, Clone)]
struct FileSystemState {
    files: IndexMap<Arc<str>, (FileType, Arc<str>)>,
}

impl FileSystemState {
    fn new() -> Self {
        Self { files: IndexMap::new() }
    }
}

#[derive(ThisError, Debug, Clone)]
pub enum FilesystemSyncError {
    #[error("'{0}': Magic number mismatch ({MAGIC_NUMBER} != {1})")]
    MagicMismatch(String, u32),
    #[error("'{0}': Checksum mismatch")]
    ChecksumMismatch(String),
    #[error("'{0}': Unsupported filesystem version: {bold}{1}{reset}", bold=*BOLD, reset=*RESET)]
    UnsupportedVersion(String, u32),
    #[error("Deserialization failure occurred with '{bold}{1}{reset}.dat': {1}", bold=*BOLD, reset=*RESET)]
    DeserializationFailure(String, String),
    #[error("Serialization failure occurred with '{0}': {1}")]
    SerializationFailure(String, String),
    #[error("Data length exceeded maximum {0} >= {1}")]
    DataLengthMaximum(u64, u64),
    #[error("Data length provided is zero")]
    DataLengthZero,
    #[error("Hash length provided is invalid.")]
    InvalidHashLength,
}

impl_error!(FilesystemSyncError);

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum FileType {
    HardLink,
    SymLink,
    Directory,
    Invalid(i8),
}

#[derive(Clone, Copy)]
pub enum SyncType {
    Filesystem,
    RefreshState,
}

impl From<i8> for FileType {
    fn from(integer: i8) -> Self {
        match integer {
            2 => Self::Directory,
            1 => Self::SymLink,
            0 => Self::HardLink,
            _ => Self::Invalid(integer),
        }
    }
}

impl From<Metadata> for FileType {
    fn from(metadata: Metadata) -> Self {
        if metadata.is_dir() {
            Self::Directory
        } else if metadata.is_symlink() {
            Self::SymLink
        } else {
            Self::HardLink
        }
    }
}

enum SyncMessage {
    LinkComplete(Arc<str>),
    SaveState(Arc<str>, FileSystemState),
}

pub struct FilesystemSync<'a> {
    state_map: HashMap<Arc<str>, FileSystemState>,
    state_map_prev: HashMap<Arc<str>, Option<FileSystemState>>,
    linked: HashSet<Arc<str>>,
    queued: HashSet<&'a str>,
    progress: Option<ProgressBar>,
    cache: &'a ContainerCache<'a>,
    pool: Option<ThreadPool>,
    max_chars: u16,
    sync_type: SyncType,
    lock: Option<&'a Lock>,
    signals: Signals,
}

impl<'a> FilesystemSync<'a> {
    pub fn new(inscache: &'a ContainerCache) -> Self {
        Self {
            pool: None,
            progress: None,
            state_map: HashMap::new(),
            state_map_prev: HashMap::new(),
            queued: HashSet::new(),
            linked: HashSet::new(),
            cache: inscache,
            max_chars: 0,
            sync_type: SyncType::Filesystem,
            lock: None,
            signals: Signals::new(SIGNAL_LIST).unwrap(),
        }
    }

    pub fn refresh_state(&mut self) {
        self.sync_type = SyncType::RefreshState;
    }

    pub fn filesystem_state(&mut self) {
        self.sync_type = SyncType::Filesystem;
    }

    pub fn assert_lock(mut self, lock: Option<&'a Lock>) -> Self {
        self.lock = lock;
        self
    }

    pub fn engage(&mut self, containers: &Vec<&'a str>) -> Result<()> {
        self.lock()?.assert()?;

        let (tx, rx) = self.link(containers, mpsc::channel())?;

        drop(tx);
        while let Ok(()) = rx.recv() {}
        self.signal()?;
        self.place_state()
    }

    fn link(
        &mut self,
        containers: &Vec<&'a str>,
        mut write_chan: (Sender<()>, Receiver<()>),
    ) -> Result<(Sender<()>, Receiver<()>)> {
        let (tx, rx): (Sender<SyncMessage>, Receiver<SyncMessage>) = mpsc::channel();

        for ins in containers {
            if self.queued.contains(ins) {
                continue;
            }

            let inshandle = self.cache.get_instance(ins)?;
            let ins_type = inshandle.metadata().container_type();
            let ins_deps = inshandle.metadata().dependencies();

            write_chan = self.link(&ins_deps, write_chan)?;

            if let Aggregate = ins_type {
                self.link_instance(inshandle, tx.clone())?;
            } else if let Base | Slice = ins_type {
                self.obtain_slice(inshandle, tx.clone())?;
            } else {
                continue;
            }

            self.queued.insert(ins);
        }

        drop(tx);
        self.wait(self.queued.clone(), rx, &write_chan);
        self.signal()?;
        Ok(write_chan)
    }

    fn wait(&mut self, mut queue: HashSet<&'a str>, rx: Receiver<SyncMessage>, write_chan: &(Sender<()>, Receiver<()>)) {
        while let Ok(recv) = rx.recv() {
            match recv {
                SyncMessage::LinkComplete(ins) => {
                    if let Some(progress) = &self.progress {
                        progress.set_message(queue_status(&self.sync_type, &queue, ins.as_ref(), self.max_chars as usize));
                        progress.inc(1);
                    }

                    queue.remove(ins.as_ref());
                    self.linked.insert(ins);
                }
                SyncMessage::SaveState(container, fs_state) => {
                    if self.state_map.contains_key(&container) {
                        continue;
                    }

                    if fs_state.files.is_empty() {
                        continue;
                    }

                    if let SyncType::Filesystem = self.sync_type {
                        self.state_map.insert(container.clone(), fs_state.clone());
                    }

                    let tx = write_chan.0.clone();

                    self.pool().unwrap().spawn(move || {
                        if let Err(err) = serialize(&format!("{}/state/{}.dat.new", *DATA_DIR, container), fs_state) {
                            eprintln_warn!("{err}");
                            drop(tx);
                        }
                    });
                }
            }
        }
    }

    fn previous_state(&mut self, instance: &Arc<str>) -> Result<Option<FileSystemState>> {
        if let Some(st) = self.state_map_prev.get(instance) {
            return Ok(st.clone());
        }

        let path = &format!("{}/state/{}.dat", *DATA_DIR, instance);
        let mut header = ByteBuffer::with_capacity(8).read();
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(err) =>
                if let IOErrorKind::NotFound = err.kind() {
                    return Ok(None);
                } else {
                    return Err(err).context_path(path)?;
                },
        };

        file.read_exact(header.as_slice_mut()).context_path(path)?;

        let magic = header.read_le_32();
        let version = header.read_le_32();

        if magic != MAGIC_NUMBER {
            Err(FilesystemSyncError::MagicMismatch(path.into(), magic))?
        }

        let (state_buffer, checksum_valid) = decode_state(file)?;

        if !checksum_valid {
            Err(FilesystemSyncError::ChecksumMismatch(path.into()))?
        }

        let buf_reader = BufReader::new(state_buffer.as_slice());
        let state = match version {
            1 | 2 => bincode_deserialize::<BufReader<&[u8]>, FileSystemState>(instance, buf_reader)?,
            3 => deserialize::<BufReader<&[u8]>, FileSystemState>(instance, buf_reader)?,
            _ => Err(FilesystemSyncError::UnsupportedVersion(path.into(), version))?,
        };

        self.state_map_prev.insert(instance.clone(), Some(state.clone()));
        Ok(Some(state))
    }

    fn blank_state(&mut self, instance: &Arc<str>) -> Option<FileSystemState> {
        self.state_map_prev.insert(instance.clone(), None);
        None
    }

    fn obtain_slice(&mut self, inshandle: &ContainerHandle, tx: Sender<SyncMessage>) -> Result<()> {
        let instance: Arc<str> = inshandle.vars().instance().into();
        let root = inshandle.vars().root().into();

        if let Err(err) = self.previous_state(&instance) {
            self.blank_state(&instance);
            err.warn();
        }

        self.pool()?.spawn(move || {
            let mut state = FileSystemState::new();

            obtain_state(root, &mut state);

            tx.send(SyncMessage::SaveState(instance.clone(), state)).unwrap();
            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        });
        Ok(())
    }

    fn link_instance(&mut self, handle: &ContainerHandle, tx: Sender<SyncMessage>) -> Result<()> {
        let mut map = Vec::new();
        let mut prev = Vec::new();
        let instance: Arc<str> = handle.vars().instance().into();
        let root: Arc<str> = handle.vars().root().into();
        let state = FileSystemState::new();

        for dep in handle.metadata().dependencies() {
            let dephandle = self.cache.get_instance(dep).unwrap();
            let state = self.state_map.get(dep).map_or_else(FileSystemState::new, |s| s.clone());
            let dep = &Arc::from(dep);
            let prev_state = match self.previous_state(dep) {
                Ok(state) => state,
                Err(err) => {
                    err.warn();
                    self.blank_state(dep)
                }
            };

            prev.push(prev_state);
            map.push((dephandle.vars().root().into(), state));
        }

        self.pool()?.spawn(move || {
            let state = filesystem_state(state, map);
            let state_prev = previous_state(prev);

            delete_files(&state, &state_prev, &root);
            delete_directories(&state, &state_prev, &root);
            link_filesystem(&state, &root);

            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        });
        Ok(())
    }

    fn pool(&self) -> Result<&ThreadPool> {
        self.pool.as_ref().map_or_else(|| Err(ErrorKind::ThreadPoolUninitialized)?, Ok)
    }

    fn lock(&self) -> Result<&Lock> {
        self.lock.map_or_else(|| Err(LockError::NotAcquired)?, Ok)
    }

    fn signal(&mut self) -> Result<()> {
        if let Err(err) = self.lock.unwrap().assert() {
            self.discard_state()?;
            Err(err)?;
        }

        if self.signals.pending().next().is_some() {
            self.discard_state()?;
            Err(SyncError::SignalInterrupt)?;
        }

        Ok(())
    }

    fn discard_state(&mut self) -> Result<()> {
        for (data, ..) in &self.state_map {
            remove_file(format!("{}/state/{data}.dat.new", *DATA_DIR)).ok();
        }

        self.lock()?.unlock()?;
        Ok(())
    }

    fn place_state(&mut self) -> Result<()> {
        for (state, ..) in &self.state_map {
            let path_old = format!("{}/state/{state}.dat", *DATA_DIR);
            let path_new = format!("{}/state/{state}.dat.new", *DATA_DIR);

            rename(&path_new, &path_old)?;
        }

        Ok(())
    }

    pub fn sync_type(&self) -> SyncType {
        self.sync_type
    }

    pub fn prepare(&mut self, length: usize, primary: Option<&ProgressBar>) {
        let size = Term::size(&Term::stdout());
        let column_half = size.1 / 2;
        let style = ProgressStyle::with_template(
            &(" {spinner:.green} {msg:<".to_owned() + column_half.to_string().as_str() + "} [{wide_bar}] {percent:<3}%"),
        )
        .unwrap()
        .progress_chars("#-")
        .tick_strings(&[">", "✓"]);
        let progress = ProgressBar::new(0).with_style(style);

        if let Some(progress) = primary {
            progress.set_style(BAR_GREEN_STYLE.clone());
            progress.set_message(format!("{}{}{}", *BOLD, self.sync_type.prepare(), *RESET));
        } else {
            println!("{} {}{}...{} ", *BAR_GREEN, *BOLD, self.sync_type.prepare(), *RESET);
        }

        progress.set_draw_target(ProgressDrawTarget::stdout());
        progress.set_message(self.sync_type.progress());
        progress.set_position(0);
        progress.set_length(length.try_into().unwrap_or(0));

        self.pool = Some(ThreadPoolBuilder::new().thread_name(|f| format!("PW-LINKER-{}", f)).build().unwrap());
        self.progress = Some(progress);
        self.max_chars = column_half - 20;
    }

    pub fn finish(&mut self, primary: Option<&ProgressBar>) {
        if let Some(progress) = primary {
            progress.set_style(BAR_CYAN_STYLE.clone());
            progress.tick();
            progress.set_draw_target(ProgressDrawTarget::stderr());
        }

        if let Some(progress) = &self.progress {
            progress.set_message(self.sync_type.finish());
            progress.finish();
        }

        self.queued.drain();
        self.linked.drain();
        self.pool = None;
        self.progress = None;
        self.max_chars = 0;
    }

    pub fn release(self) {
        drop(self);
    }
}

impl SyncType {
    pub fn prepare(&self) -> &str {
        match self {
            Self::Filesystem => "Synchronizing container filesystems",
            Self::RefreshState => "Refreshing filesystem state data",
        }
    }

    pub fn progress(&self) -> &'static str {
        match self {
            Self::Filesystem => "Synchronizing filesystems..",
            Self::RefreshState => "Refreshing state..",
        }
    }

    pub fn finish(&self) -> &'static str {
        match self {
            Self::Filesystem => "Synchronization complete.",
            Self::RefreshState => "Refresh complete.",
        }
    }
}

pub fn validate_fs_states<'a>(instances: &'a Vec<&'a str>) -> bool {
    for ins in instances {
        if !match check(ins) {
            Ok(bool) => !bool,
            Err(err) => {
                err.warn();
                false
            }
        } {
            return false;
        }
    }

    true
}

pub fn create_blank_state(container: &str) -> Result<()> {
    serialize(&format!("{}/state/{}.dat", *DATA_DIR, container), FileSystemState::new())
}

fn deserialize<R: Read, T: for<'de> Deserialize<'de>>(instance: &str, mut reader: R) -> Result<T> {
    let mut bytes = Vec::new();

    reader.read_to_end(&mut bytes)?;

    match postcard::from_bytes(&bytes) {
        Ok(state) => Ok(state),
        Err(err) => Err(FilesystemSyncError::DeserializationFailure(instance.into(), err.to_string()))?,
    }
}

fn bincode_deserialize<R: Read, T: for<'de> Deserialize<'de>>(instance: &str, reader: R) -> Result<T> {
    match bincode::options()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(BYTE_LIMIT)
        .deserialize_from::<R, T>(reader)
    {
        Ok(state) => Ok(state),
        Err(err) => Err(FilesystemSyncError::DeserializationFailure(instance.into(), err.to_string()))?,
    }
}

fn serialize(path: &str, ds: FileSystemState) -> Result<()> {
    let mut hasher = Sha256::new();
    let state_data = match postcard::to_allocvec(&ds) {
        Ok(vec) => vec,
        Err(err) => Err(FilesystemSyncError::SerializationFailure(path.into(), err.to_string()))?,
    };

    copy(&mut state_data.as_slice(), &mut hasher).context_path(path)?;
    encode_state(path, state_data, hasher.finalize().to_vec()).context_path(path)?;
    Ok(())
}

fn decode_state<R: Read>(mut stream: R) -> Result<(Vec<u8>, bool)> {
    let mut header_buffer = ByteBuffer::with_capacity(10).read();

    stream.read_exact(header_buffer.as_slice_mut())?;

    let hash_length = header_buffer.read_le_16();
    let state_length = header_buffer.read_le_64();

    if state_length == 0 {
        Err(FilesystemSyncError::DataLengthZero)?;
    } else if hash_length != 32 {
        Err(FilesystemSyncError::InvalidHashLength)?;
    } else if state_length >= BYTE_LIMIT {
        Err(FilesystemSyncError::DataLengthMaximum(state_length, BYTE_LIMIT))?;
    }

    let mut hash_buffer = vec![0; hash_length as usize];
    let mut state_buffer = vec![0; state_length as usize];

    stream.read_exact(&mut hash_buffer)?;

    let mut hasher = Sha256::new();
    let mut reader = Decoder::new(stream)?;

    reader.read_exact(&mut state_buffer)?;
    copy(&mut state_buffer.as_slice(), &mut hasher)?;

    Ok((state_buffer, hasher.finalize().to_vec() == hash_buffer))
}

fn encode_state(path: &str, state_data: Vec<u8>, hash: Vec<u8>) -> IOResult<u64> {
    let mut output = File::create(path)?;
    let mut header = ByteBuffer::new().write();

    header.write_le_32(MAGIC_NUMBER);
    header.write_le_32(VERSION);
    header.write_le_16(hash.len() as u16);
    header.write_le_64(state_data.len() as u64);
    output.write_all(header.as_slice())?;
    output.write_all(&hash)?;
    copy(&mut state_data.as_slice(), &mut Encoder::new(output, 3)?.auto_finish())
}

fn check(instance: &str) -> Result<bool> {
    let path = &format!("{}/state/{}.dat", *DATA_DIR, instance);
    let mut header_buffer = ByteBuffer::with_capacity(8).read();
    let mut file = File::open(path).context_path(path)?;

    file.read_exact(header_buffer.as_slice_mut())?;

    let magic = header_buffer.read_le_32();
    let version = header_buffer.read_le_32();

    Ok(magic != MAGIC_NUMBER || version != VERSION)
}

fn previous_state(map: Vec<Option<FileSystemState>>) -> FileSystemState {
    let mut state = FileSystemState::new();

    for ins_state in map.into_iter().flatten() {
        state.files.extend(ins_state.files);
    }

    state
}

fn filesystem_state(mut state: FileSystemState, map: Vec<(Arc<str>, FileSystemState)>) -> FileSystemState {
    for ins_state in map {
        if ins_state.1.files.is_empty() {
            obtain_state(ins_state.0, &mut state);
        } else {
            state.files.extend(ins_state.1.files);
        }
    }

    state
}

fn obtain_state(root: Arc<str>, state: &mut FileSystemState) {
    let len = root.len();
    let entries = WalkDir::new(root.as_ref()).into_iter().filter_map(|e| e.ok());

    for entry in entries {
        let src: Arc<str> = entry.path().to_str().unwrap().into();
        let src_tr: Arc<str> = src.split_at(len).1.into();

        if state.files.get(&src_tr).is_some() {
            continue;
        }

        if src.contains("/var/lib/pacman") || src.ends_with("/etc/ld.so.cache") {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        state.files.insert(src_tr, (FileType::from(metadata), src));
    }
}

fn link_filesystem(state: &FileSystemState, root: &str) {
    state.files.par_iter().filter(|a| a.1.0 != FileType::Directory).for_each(|file| {
        let path = &format!("{}{}", root, file.0);

        if let FileType::SymLink = file.1.0 {
            if let Err(error) = create_soft_link(&file.1.1, path).context_path(path) {
                eprintln_warn!("Failed to symlink {path:?}: {error}");
            }
        } else if let FileType::HardLink = file.1.0 {
            if let Err(error) = create_hard_link(&file.1.1, path).context_path(path) {
                eprintln_warn!("Failed to hardlink {path:?}: {error}");
            }
        }
    });
}

fn delete_files(state: &FileSystemState, state_res: &FileSystemState, root: &str) {
    let (tx, rx) = mpsc::sync_channel(0);
    let tx_clone: mpsc::SyncSender<()> = tx.clone();

    state_res.files.par_iter().filter(|a| a.1.0 != FileType::Directory).for_each(|file| {
        let _ = tx_clone;

        if state.files.get(file.0).is_none() {
            let path_str = &format!("{}{}", root, file.0);
            let path = Path::new(path_str);

            if let FileType::SymLink = file.1.0 {
                if let Err(error) = remove_symlink(path).context_path(path) {
                    eprintln_warn!("Failed to remove symlink: {error}");
                }
            } else if let (true, FileType::HardLink) = (path.exists(), &file.1.0) {
                if let Err(error) = remove_file(path).context_path(path) {
                    eprintln_warn!("Failed to remove file: {error}'");
                }
            }
        }
    });

    drop(tx);
    rx.try_iter();
}

fn delete_directories(state: &FileSystemState, state_res: &FileSystemState, root: &str) {
    let (tx, rx) = mpsc::sync_channel(0);
    let tx_clone: mpsc::SyncSender<()> = tx.clone();

    state_res.files.par_iter().for_each(move |file| {
        let _ = tx_clone;

        if state.files.get(file.0).is_none() {
            let path: &str = &format!("{}{}", root, file.0);
            let path = Path::new(path);

            if !path.exists() {
                return;
            }

            if let FileType::Directory = file.1.0 {
                remove_dir_all(path).ok();
            }
        }
    });

    drop(tx);
    rx.try_iter();
}

fn create_soft_link(src: &str, dest: &str) -> IOResult<()> {
    let dest_path = Path::new(&dest);
    let src_path = fs::read_link(src)?;

    if let Ok(src_path_dest) = fs::read_link(dest_path) {
        if src_path.as_path() == src_path_dest.as_path() {
            return Ok(());
        }
    }

    if dest_path.is_dir() {
        remove_dir_all(dest_path)
    } else if dest_path.exists() {
        remove_file(dest_path)
    } else {
        remove_symlink(dest_path)
    }?;

    if let Some(path) = dest_path.parent() {
        if !path.exists() {
            create_dir_all(path)?;
        }
    }

    symlink(&src_path, dest_path)
}

pub fn create_hard_link(src: &str, dest: &str) -> IOResult<()> {
    let src_path = Path::new(&src);
    let dest_path = Path::new(&dest);

    if !src_path.exists() {
        Err(IOErrorKind::NotFound)?
    }

    if !dest_path.exists() {
        if let Some(path) = dest_path.parent() {
            if !path.exists() {
                remove_symlink(path)?;
                create_dir_all(path)?;
            }
        }

        remove_symlink(dest_path)?;
        hard_link(src_path, dest_path)
    } else {
        let meta_dest = metadata(dest_path)?;
        let meta_src = metadata(src_path)?;

        if meta_src.ino() != meta_dest.ino() {
            if meta_dest.is_dir() {
                remove_dir_all(dest_path)
            } else {
                remove_file(dest_path)
            }?;

            hard_link(src_path, dest_path)?;
        }

        Ok(())
    }
}

#[inline]
fn remove_symlink(path: &Path) -> IOResult<()> {
    if fs::read_link(path).is_ok() {
        remove_file(path)?
    }

    Ok(())
}

fn queue_status(sync_type: &SyncType, queue: &HashSet<&str>, compare: &str, max_chars: usize) -> String {
    let mut char_amt = 0;
    let mut diff = 0;
    let mut string = String::new();
    let mut strs: Vec<&str> = Vec::new();

    for contrast in queue {
        let contrast: &str = contrast;

        if compare == contrast {
            continue;
        }

        char_amt += contrast.len();

        if char_amt >= max_chars - contrast.len() {
            diff = queue.len() - strs.len();
            break;
        }

        strs.push(contrast);
    }

    for idx in 0 .. strs.len() {
        let str = strs.get(idx).unwrap();

        if idx > 0 {
            string.push_str(format!(", {str}").as_str());
        } else {
            string.push_str(str.to_string().as_str());
        }
    }

    if diff > 0 {
        string.push_str(format!(", and {diff} more..").as_str());
    }

    if string.is_empty() {
        string.push_str(sync_type.progress());
    }

    string
}
