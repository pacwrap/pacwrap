/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
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
    fmt::{Display, Error as FmtError, Formatter},
    fs::{self, create_dir_all, hard_link, metadata, remove_dir_all, remove_file, rename, File, Metadata},
    io::{copy, BufReader, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult, Write},
    os::unix::{fs::symlink, prelude::MetadataExt},
    path::Path,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
};

use bincode::Options;
use dialoguer::console::Term;
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use signal_hook::iterator::Signals;
use walkdir::WalkDir;
use zstd::{Decoder, Encoder};

use crate::{
    config::{ContainerCache, ContainerHandle, ContainerType::*},
    constants::{BAR_GREEN, BOLD, DATA_DIR, RESET, SIGNAL_LIST},
    err,
    impl_error,
    sync::SyncError,
    utils::bytebuffer::ByteBuffer,
    Error,
    ErrorGeneric,
    ErrorKind,
    ErrorTrait,
};

const VERSION: u32 = 2;
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

#[derive(Debug, Clone)]
pub enum FilesystemSyncError {
    MagicMismatch(String, u32),
    ChecksumMismatch(String),
    UnsupportedVersion(String, u32),
    DeserializationFailure(String, String),
    SerializationFailure(String, String),
}

impl_error!(FilesystemSyncError);

impl Display for FilesystemSyncError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            Self::SerializationFailure(file, err) => write!(fmter, "Serialization failure occurred with '{file}': {err}"),
            Self::UnsupportedVersion(file, ver) =>
                write!(fmter, "'{file}': Unsupported filesystem version: {}{ver}{}", *BOLD, *RESET),
            Self::DeserializationFailure(file, err) =>
                write!(fmter, "Deserialization failure occurred with '{}{file}{}.dat': {err}", *BOLD, *RESET),
            Self::ChecksumMismatch(file) => write!(fmter, "'{file}': Checksum mismatch"),
            Self::MagicMismatch(file, magic) => write!(fmter, "'{file}': Magic number mismatch ({MAGIC_NUMBER} != {magic})"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum FileType {
    HardLink,
    SymLink,
    Directory,
    Invalid(i8),
}

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
            signals: Signals::new(SIGNAL_LIST).unwrap(),
        }
    }

    pub fn refresh_state(&mut self) {
        self.sync_type = SyncType::RefreshState;
    }

    pub fn filesystem_state(&mut self) {
        self.sync_type = SyncType::Filesystem;
    }

    pub fn engage(&mut self, containers: &Vec<&'a str>) -> Result<(), Error> {
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
    ) -> Result<(Sender<()>, Receiver<()>), Error> {
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
                SyncMessage::SaveState(dep, fs_state) => {
                    if let Some(_) = self.state_map.get(&dep) {
                        continue;
                    }

                    if fs_state.files.len() == 0 {
                        continue;
                    }

                    if let SyncType::Filesystem = self.sync_type {
                        self.state_map.insert(dep.clone(), fs_state.clone());
                    }

                    let tx = write_chan.0.clone();

                    self.pool().unwrap().spawn(move || {
                        if let Err(err) = serialize(dep, fs_state) {
                            err.warn();
                            drop(tx);
                        }
                    });
                }
            }
        }
    }

    fn previous_state(&mut self, instance: &Arc<str>) -> Result<Option<FileSystemState>, Error> {
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
                    return Err(err).prepend_io(|| path.into());
                },
        };

        file.read_exact(header.as_slice_mut()).prepend_io(|| path.into())?;

        let magic = header.read_le_32();
        let version = header.read_le_32();

        if magic != MAGIC_NUMBER {
            err!(FilesystemSyncError::MagicMismatch(path.into(), magic))?
        } else if version != VERSION {
            let state = match version {
                1 => deserialize::<File, FileSystemState>(&instance, file)?,
                _ => err!(FilesystemSyncError::UnsupportedVersion(path.into(), version))?,
            };

            self.state_map_prev.insert(instance.clone(), Some(state.clone()));
            Ok(Some(state))
        } else {
            let (state_buffer, checksum_valid) = decode_state(file).prepend_io(|| path.into())?;

            if !checksum_valid {
                err!(FilesystemSyncError::ChecksumMismatch(path.into()))?
            }

            let buf_reader = BufReader::new(state_buffer.as_slice());
            let state = deserialize::<BufReader<&[u8]>, FileSystemState>(&instance, buf_reader)?;

            self.state_map_prev.insert(instance.clone(), Some(state.clone()));
            Ok(Some(state))
        }
    }

    fn blank_state(&mut self, instance: &Arc<str>) -> Option<FileSystemState> {
        self.state_map_prev.insert(instance.clone(), None);
        None
    }

    fn obtain_slice(&mut self, inshandle: &ContainerHandle, tx: Sender<SyncMessage>) -> Result<(), Error> {
        let instance: Arc<str> = inshandle.vars().instance().into();
        let root = inshandle.vars().root().into();

        if let Err(err) = self.previous_state(&instance) {
            self.blank_state(&instance);
            err.warn();
        }

        Ok(self.pool()?.spawn(move || {
            let mut state = FileSystemState::new();

            obtain_state(root, &mut state);

            tx.send(SyncMessage::SaveState(instance.clone(), state)).unwrap();
            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        }))
    }

    fn link_instance(&mut self, inshandle: &ContainerHandle, tx: Sender<SyncMessage>) -> Result<(), Error> {
        let mut map = Vec::new();
        let mut prev = Vec::new();
        let instance: Arc<str> = inshandle.vars().instance().into();
        let root: Arc<str> = inshandle.vars().root().into();
        let state = FileSystemState::new();

        for dep in inshandle.metadata().dependencies() {
            let dephandle = self.cache.get_instance(dep).unwrap();
            let state = match self.state_map.get(dep) {
                Some(state) => state.clone(),
                None => FileSystemState::new(),
            };
            let dep = &Arc::from(dep.as_ref());
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

        Ok(self.pool()?.spawn(move || {
            let state = filesystem_state(state, map);
            let state_prev = previous_state(prev);

            delete_files(&state, &state_prev, &root);
            delete_directories(&state, &state_prev, &root);
            link_filesystem(&state, &root);

            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        }))
    }

    fn pool(&self) -> Result<&ThreadPool, Error> {
        match self.pool.as_ref() {
            Some(pool) => Ok(pool),
            None => err!(ErrorKind::ThreadPoolUninitialized),
        }
    }

    fn signal(&mut self) -> Result<(), Error> {
        for _ in self.signals.pending() {
            for (data, ..) in &self.state_map {
                remove_file(&format!("{}/state/{data}.dat.new", *DATA_DIR)).ok();
            }

            err!(SyncError::SignalInterrupt)?;
        }

        Ok(())
    }

    fn place_state(&mut self) -> Result<(), Error> {
        for (state, ..) in &self.state_map {
            let path_old = format!("{}/state/{state}.dat", *DATA_DIR);
            let path_new = format!("{}/state/{state}.dat.new", *DATA_DIR);

            rename(&path_new, &path_old).prepend_io(|| path_new)?;
        }

        Ok(())
    }

    pub fn prepare(&mut self, length: usize) {
        let size = Term::size(&Term::stdout());
        let column_half = size.1 / 2;
        let style = ProgressStyle::with_template(
            &(" {spinner:.green} {msg:<".to_owned() + column_half.to_string().as_str() + "} [{wide_bar}] {percent:<3}%"),
        )
        .unwrap()
        .progress_chars("#-")
        .tick_strings(&[">", "✓"]);
        let progress = ProgressBar::new(0).with_style(style);

        println!("{} {}{}...{} ", *BAR_GREEN, *BOLD, self.sync_type.prepare(), *RESET);
        progress.set_draw_target(ProgressDrawTarget::stdout());
        progress.set_message(self.sync_type.progress());
        progress.set_position(0);
        progress.set_length(length.try_into().unwrap_or(0));

        self.pool = Some(ThreadPoolBuilder::new().thread_name(|f| format!("PW-LINKER-{}", f)).build().unwrap());
        self.progress = Some(progress);
        self.max_chars = column_half - 20;
    }

    pub fn finish(&mut self) {
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
    fn prepare(&self) -> &str {
        match self {
            Self::Filesystem => "Synchronizing container filesystems",
            Self::RefreshState => "Refreshing filesystem state data",
        }
    }

    fn progress(&self) -> &'static str {
        match self {
            Self::Filesystem => "Synchronizing filesystems..",
            Self::RefreshState => "Refreshing state..",
        }
    }

    fn finish(&self) -> &'static str {
        match self {
            Self::Filesystem => "Synchronization complete.",
            Self::RefreshState => "Refresh complete.",
        }
    }
}

pub fn validate_fs_states<'a>(vec: &'a Vec<&'a str>) -> bool {
    for instance in vec {
        match check(instance) {
            Ok(bool) =>
                if bool {
                    return false;
                },
            Err(err) => {
                err.warn();
                return false;
            }
        }
    }

    true
}

pub fn create_blank_state(container: &str) -> Result<(), Error> {
    serialize(container.into(), FileSystemState::new())
}

fn deserialize<R: Read, T: for<'de> Deserialize<'de>>(instance: &str, reader: R) -> Result<T, Error> {
    match bincode::options()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(BYTE_LIMIT)
        .deserialize_from::<R, T>(reader)
    {
        Ok(state) => Ok(state),
        Err(err) => err!(FilesystemSyncError::DeserializationFailure(instance.into(), err.to_string())),
    }
}

fn serialize(dep: Arc<str>, ds: FileSystemState) -> Result<(), Error> {
    let error_path = &format!("'{}{}{}.dat.new'", *BOLD, dep, *RESET);
    let path = &format!("{}/state/{}.dat.new", *DATA_DIR, dep);
    let mut hasher = Sha256::new();
    let mut state_data = Vec::new();

    if let Err(err) = bincode::options()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(BYTE_LIMIT)
        .serialize_into(&mut state_data, &ds)
    {
        err!(FilesystemSyncError::SerializationFailure(error_path.into(), err.as_ref().to_string()))?
    }

    copy(&mut state_data.as_slice(), &mut hasher).prepend_io(|| error_path.into())?;
    encode_state(path, state_data, hasher.finalize().to_vec()).prepend_io(|| error_path.into())?;
    Ok(())
}

fn decode_state<'a, R: Read>(mut stream: R) -> IOResult<(Vec<u8>, bool)> {
    let mut header_buffer = ByteBuffer::with_capacity(10).read();

    stream.read_exact(&mut header_buffer.as_slice_mut())?;

    let hash_length = header_buffer.read_le_16();
    let state_length = header_buffer.read_le_64();

    if state_length >= BYTE_LIMIT {
        Err(IOError::new(
            IOErrorKind::InvalidInput,
            format!("Data length provided exceeded maximum {state_length} >= {BYTE_LIMIT}"),
        ))?;
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
    output.write(header.as_slice())?;
    output.write(&hash)?;
    copy(&mut state_data.as_slice(), &mut Encoder::new(output, 3)?.auto_finish())
}

fn check(instance: &str) -> Result<bool, Error> {
    let path = &format!("{}/state/{}.dat", *DATA_DIR, instance);
    let mut header_buffer = ByteBuffer::with_capacity(8).read();
    let mut file = File::open(path).prepend_io(|| path.into())?;

    file.read_exact(header_buffer.as_slice_mut()).prepend_io(|| path.into())?;

    let magic = header_buffer.read_le_32();
    let version = header_buffer.read_le_32();

    Ok(magic != MAGIC_NUMBER || version != VERSION)
}

fn previous_state(map: Vec<Option<FileSystemState>>) -> FileSystemState {
    let mut state = FileSystemState::new();

    for ins_state in map {
        if let Some(ins_state) = ins_state {
            state.files.extend(ins_state.files);
        }
    }

    state
}

fn filesystem_state(mut state: FileSystemState, map: Vec<(Arc<str>, FileSystemState)>) -> FileSystemState {
    for ins_state in map {
        if ins_state.1.files.len() == 0 {
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

        if let Some(_) = state.files.get(&src_tr) {
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
    state.files.par_iter().filter(|a| a.1 .0 != FileType::Directory).for_each(|file| {
        let path = &format!("{}{}", root, file.0);

        if let FileType::SymLink = file.1 .0 {
            if let Err(error) = create_soft_link(&file.1 .1, path).prepend(|| format!("Failed to symlink '{path}'")) {
                error.warn();
            }
        } else if let FileType::HardLink = file.1 .0 {
            if let Err(error) = create_hard_link(&file.1 .1, path).prepend(|| format!("Failed to hardlink '{path}'")) {
                error.warn();
            }
        }
    });
}

fn delete_files(state: &FileSystemState, state_res: &FileSystemState, root: &str) {
    let (tx, rx) = mpsc::sync_channel(0);
    let tx_clone: mpsc::SyncSender<()> = tx.clone();

    state_res.files.par_iter().filter(|a| a.1 .0 != FileType::Directory).for_each(|file| {
        let _ = tx_clone;

        if let None = state.files.get(file.0) {
            let path_str = &format!("{}{}", root, file.0);
            let path = Path::new(path_str);

            if let FileType::SymLink = file.1 .0 {
                if let Err(error) = remove_symlink(path).prepend(|| format!("Failed to remove symlink '{path_str}'")) {
                    error.warn();
                }
            } else if let (true, FileType::HardLink) = (path.exists(), &file.1 .0) {
                if let Err(error) = remove_file(path).prepend(|| format!("Failed to remove file '{path_str}'")) {
                    error.warn();
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

        if let None = state.files.get(file.0) {
            let path: &str = &format!("{}{}", root, file.0);
            let path = Path::new(path);

            if !path.exists() {
                return;
            }

            if let FileType::Directory = file.1 .0 {
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
            create_dir_all(&path)?;
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
                remove_symlink(&path)?;
                create_dir_all(&path)?;
            }
        }

        remove_symlink(dest_path)?;
        hard_link(src_path, dest_path)
    } else {
        let meta_dest = metadata(&dest_path)?;
        let meta_src = metadata(&src_path)?;

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
    if let Ok(_) = fs::read_link(path) {
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
        let contrast: &str = contrast.as_ref();

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
            string.push_str(format!("{str}").as_str());
        }
    }

    if diff > 0 {
        string.push_str(format!(", and {diff} more..").as_str());
    }

    if string.len() == 0 {
        string.push_str(sync_type.progress());
    }

    string
}
