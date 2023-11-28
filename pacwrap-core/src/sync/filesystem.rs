use std::fmt;
use std::fs::{self, File, Metadata};
use std::os::unix::fs::symlink;
use std::os::unix::prelude::MetadataExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Sender, self, Receiver};

use dialoguer::console::Term;
use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use serde::{Serialize, Deserialize, Deserializer, Serializer, de::Visitor};
use walkdir::WalkDir;
use std::collections::{HashMap, HashSet};

use crate::ErrorKind;
use crate::config::{InstanceHandle, InstanceCache, InstanceType::*};
use crate::constants::{RESET, BOLD, ARROW_CYAN, BAR_GREEN, DATA_DIR};
use crate::utils::{print_warning, print_error};

impl Serialize for FileType {
    fn serialize<D: Serializer>(&self, serializer: D) -> Result<D::Ok, D::Error> 
    where D: serde::Serializer {
        serializer.serialize_i64(self.as_integer())
    }
}


impl <'de>Deserialize<'de> for FileType {
    fn deserialize<D: Deserializer<'de>>(serializer: D) -> Result<Self, D::Error> 
    where D: serde::Deserializer<'de> {
        serializer.deserialize_i64(FileTypeVisitor)
    }
}

struct FileTypeVisitor;

impl<'de> Visitor<'de> for FileTypeVisitor {
    type Value = FileType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an integer between `0` and `2`")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where E: serde::de::Error, {
        let value = v.into();

        if let FileType::Invalid(v) = value {
            Err(E::invalid_value(serde::de::Unexpected::Signed(v), &self))?
        }

        Ok(value)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct FileSystemState {
    files: IndexMap<Arc<str>, (FileType, Arc<str>)>
}

impl FileSystemState {
    fn new() -> Self {
        Self {
            files: IndexMap::new()
        }
    }
}

#[derive(Clone)]
enum FileType {
    HardLink,
    SymLink,
    Directory,
    Invalid(i64),
}

impl From<i64> for FileType {
    fn from(integer: i64) -> Self {
        match integer {
            2 => Self::Directory,
            1 => Self::SymLink,
            0 => Self::HardLink,
            _ => Self::Invalid(integer)
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

impl FileType {
    fn as_integer(&self) -> i64 {
        match self {
            Self::Directory => 2,
            Self::SymLink => 1,
            Self::HardLink => 0,
            Self::Invalid(i) => *i,
        }
    }
}

enum SyncMessage {
    LinkComplete(Arc<str>),
    SaveState(Arc<str>, FileSystemState),
}

#[allow(dead_code)]
pub struct FileSystemStateSync<'a> {
    state_map: HashMap<Arc<str>, FileSystemState>, 
    state_map_prev: HashMap<Arc<str>, FileSystemState>,
    linked: HashSet<Arc<str>>,
    queued: HashSet<&'a str>,
    progress: ProgressBar,
    cache: &'a InstanceCache<'a>,
    pool: Option<ThreadPool>,
    max_chars: u16, 
}

impl <'a>FileSystemStateSync<'a> { 
    pub fn new(inscache: &'a InstanceCache) -> Self {
        let size = Term::size(&Term::stdout());
        let column_half = size.1 / 2;
        let style = ProgressStyle::with_template(&(" {spinner:.green} {msg:<".to_owned()
            +column_half.to_string().as_str()+"} [{wide_bar}] {percent:<3}%"))
            .unwrap()
            .progress_chars("#-")
            .tick_strings(&[">", "✓"]); 
        let pr = ProgressBar::new(0).with_style(style);
        
        pr.set_draw_target(ProgressDrawTarget::hidden());

        Self {
            pool: None,
            progress: pr,
            state_map: HashMap::new(),
            state_map_prev: HashMap::new(),
            queued: HashSet::new(),
            linked: HashSet::new(),
            cache: inscache,
            max_chars: column_half - 20,
        }
    }

    pub fn engage(&mut self, containers: &Vec<&'a str>) -> Result<(), ErrorKind> {
        let (tx, rx) = self.link(&containers, mpsc::channel())?; 
        
        drop(tx); 
        while let Ok(()) = rx.recv() {}
        Ok(())
    }
  
    fn link(&mut self, containers: &Vec<&'a str>, mut write_chan: (Sender<()>, Receiver<()>)) -> Result<(Sender<()>, Receiver<()>), ErrorKind> { 
        let (tx, rx): (Sender<SyncMessage>, Receiver<SyncMessage>) = mpsc::channel();

        for ins in containers { 
            if self.queued.contains(ins) {
                continue;
            }

            let inshandle = match self.cache.get_instance(ins) {
                Some(ins) => ins,
                None => Err(ErrorKind::InstanceNotFound(ins.to_string()))?
            };
            let deps = inshandle.metadata()
                .dependencies()
                .iter()
                .map(|a| a.as_ref())
                .collect();
          
            write_chan = self.link(&deps, write_chan)?;
            
            if let ROOT = inshandle.metadata().container_type() {
                self.link_instance(inshandle, tx.clone())?; 
            } else {
                self.obtain_slice(inshandle, tx.clone())?; 
            }

            self.queued.insert(ins);
        }

        drop(tx);
        self.wait(self.queued.clone(), rx, &write_chan);
        Ok(write_chan)
    }

    fn wait(&mut self, mut queue: HashSet<&'a str>, rx: Receiver<SyncMessage>, write_chan: &(Sender<()>, Receiver<()>)) { 
        while let Ok(recv) = rx.recv() {
            match recv {
                SyncMessage::LinkComplete(ins) => {
                    let instance = ins.as_ref();
                    let status = queue_status(&queue, instance, self.max_chars as usize);
                    
                    queue.remove(instance);
                    self.linked.insert(ins);
                    self.progress.set_message(status);
                    self.progress.inc(1);
                },
                SyncMessage::SaveState(dep, fs_state) => {
                    if let Some(_) = self.state_map.get(&dep) {
                        continue;
                    }

                    if fs_state.files.len() == 0 {
                        continue;
                    }
                
                    self.state_map.insert(dep.clone(), fs_state.clone());
                    self.write(write_chan.0.clone(), fs_state, dep);   
                }
            }
        } 
    }

    fn previous_state(&mut self, instance: &Arc<str>) -> FileSystemState {
        if let Some(st) = self.state_map_prev.get(instance) {
            return st.clone()
        }

        let path = format!("{}/state/{}.dat", *DATA_DIR, instance);
        let file = match File::open(&path) { 
            Ok(file) => file,
            Err(err) => {
                let state = FileSystemState::new(); 
      
                if err.kind() != std::io::ErrorKind::NotFound {
                    print_warning(format!("Reading '{}': {}", path, err.kind()));
                }

                self.state_map_prev.insert(instance.clone(), state.clone()); 
                return state
            },
        };

        match ciborium::from_reader::<FileSystemState, File>(file) {
            Ok(state) => { 
                self.state_map_prev.insert(instance.clone(), state.clone());
                state
            },
            Err(err) => { 
                let state = FileSystemState::new();

                if let ciborium::de::Error::Semantic(_, error) = err {
                    print_error(format!("Deserialization failure occurred with '{}{instance}{}.dat': {error}", *BOLD, *RESET)); 
                }

                self.state_map_prev.insert(instance.clone(), state.clone()); 
                state
            }
        }
    }

    fn write(&mut self, tx: Sender<()>, ds: FileSystemState, dep: Arc<str>) {
        let path: &str = &format!("{}/state/{}.dat", *DATA_DIR, dep);
        let output = match File::create(path) {
            Ok(file) => file,
            Err(err) => {
                print_warning(format!("Writing '{}': {}", path, err.kind()));  
                return;
            }
        };

        self.pool().unwrap().spawn(move ||{ 
            if let Err(err) = ciborium::into_writer(&ds, output) {
                print_error(format!("Serialization failure occurred with '{}{dep}{}.dat': {}", *BOLD, *RESET, err.to_string())); 
            }

            drop(tx);
        });
    } 

    fn obtain_slice(&mut self, inshandle: &InstanceHandle, tx: Sender<SyncMessage>) -> Result<(), ErrorKind> {
        let instance: Arc<str> = inshandle.vars().instance().into();
        let root = inshandle.vars().root().into();
       
        self.previous_state(&instance);
        Ok(self.pool()?.spawn(move ||{ 
            let mut state = FileSystemState::new();

            obtain_state(root, &mut state);

            tx.send(SyncMessage::SaveState(instance.clone(), state)).unwrap();
            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        }))
    }

    fn link_instance(&mut self, inshandle: &InstanceHandle, tx: Sender<SyncMessage>) -> Result<(), ErrorKind> {
        let mut map = Vec::new(); 
        let mut prev = Vec::new();
        let deps = inshandle.metadata().dependencies(); 
        let instance: Arc<str> = inshandle.vars().instance().into();
        let root: Arc<str> = inshandle.vars().root().into();
        let state = FileSystemState::new();
 
        for dep in deps {
            let dephandle = self.cache.get_instance(dep).unwrap();
            let state = match self.state_map.get(dep.as_ref().into()) { 
                Some(state) => state.clone(),
                None => FileSystemState::new()
            };

            prev.push(self.previous_state(&Arc::from(dep.as_ref())));
            map.push((dephandle.vars().root().into(), state));
        }

        Ok(self.pool()?.spawn(move ||{ 
            let state = filesystem_state(state, map);
            let state_prev = previous_state(prev);

            delete_files(&state, &state_prev, &root);
            delete_directories(&state, &state_prev, &root);
            link_filesystem(&state, &root);

            tx.send(SyncMessage::LinkComplete(instance)).unwrap();
        }))
    }

    fn pool(&self) -> Result<&ThreadPool, ErrorKind> {
        match self.pool.as_ref() {
          Some(pool) =>  Ok(pool),
          None => Err(ErrorKind::ThreadPoolUninitialized)
        }
    }

    pub fn prepare_single(&mut self) {
        println!("{} Synchronizing container state...", *ARROW_CYAN);     

        if let None = self.pool {
            self.pool = Some(ThreadPoolBuilder::new()
                .thread_name(|f| { format!("PW-LINKER-{}", f) })
                .num_threads(2)
                .build()
                .unwrap());  
        }
    }

    pub fn prepare(&mut self, length: usize) {
        println!("{} {}Synchronizing container filesystems...{} ",*BAR_GREEN, *BOLD, *RESET);  

        self.pool = Some(ThreadPoolBuilder::new()
            .thread_name(|f| { format!("PW-LINKER-{}", f) })
            .build()
            .unwrap());
        self.progress.set_draw_target(ProgressDrawTarget::stdout());
        self.progress.set_message("Synhcronizing containers..");
        self.progress.set_position(0);
        self.progress.set_length(length.try_into().unwrap_or(0));
    }

    pub fn set_cache(&mut self, inscache: &'a InstanceCache) {
        self.cache = inscache;
    }

    pub fn finish(&mut self) {
        self.progress.set_message("Synchronization complete."); 
        self.progress.finish();
        self.pool = None;
    }

    pub fn release(self) -> Option<FileSystemStateSync<'a>> {
        drop(self);
        None
    }
}

fn previous_state(map: Vec<FileSystemState>) -> FileSystemState {
    let mut state = FileSystemState::new();

    for ins_state in map {
        state.files.extend(ins_state.files);
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
    let entries = WalkDir::new(root.as_ref())
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in entries { 
        let src: Arc<str> = entry.path().to_str().unwrap().into();
        let src_tr: Arc<str> = src.split_at(len).1.into();
                    
        if let Some(_) = state.files.get(&src_tr) {
            continue;
        }

        if src.contains("/var/lib/pacman")
        || src.ends_with("/etc/ld.so.cache") {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(meta) => meta, Err(_) => continue
        };

        state.files.insert(src_tr, (FileType::from(metadata), src));
    }
}

fn link_filesystem(state: &FileSystemState, root: &str) {
    state.files.par_iter().for_each(|file| {
        if let FileType::SymLink = file.1.0 {
            if let Err(error) = create_soft_link(&file.1.1, &format!("{}{}", root, file.0)) {
                print_warning(error);
            }
        } else if let FileType::HardLink = file.1.0 {
            if let Err(error) = create_hard_link(&file.1.1, &format!("{}{}", root, file.0)) {
                print_warning(error);
            }
        }
    });
}
 
fn delete_files(state: &FileSystemState, state_res: &FileSystemState, root: &str) { 
    let (tx, rx) = mpsc::sync_channel(0);
    let tx_clone: mpsc::SyncSender<()> = tx.clone();

    state_res.files.par_iter().for_each(|file| { 
        let _ = tx_clone; 
 
        if let None = state.files.get(file.0) {
            let path: &str = &format!("{}{}", root, file.0);
            let path = Path::new(path); 

            if ! path.exists() {
                if let FileType::SymLink = file.1.0 {
                    if let Err(error) = remove_symlink(path) {
                        print_warning(error);
                    }
                }
                return;
            }

            if let FileType::HardLink = file.1.0 {
                if let Err(error) = remove_file(path) { 
                    print_warning(error); 
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
                
            if ! path.exists() {
                return;
            }
 
            if let FileType::Directory = file.1.0 { 
                remove_directory(path).ok();
            }
        }
    });

    drop(tx);
    rx.try_iter();
}

fn create_soft_link(src: &str, dest: &str) -> Result<(),String> {   
    let dest_path = Path::new(&dest);
    let src_path = match fs::read_link(src) {
        Ok(path) => path,
        Err(err) => Err(format!("Source symlink '{}' {} ", src, err.to_string()))?,
    };

    if let Ok(src_path_dest) = fs::read_link(dest_path) {
        if src_path.as_path() == src_path_dest.as_path() {
            return Ok(());
        }
    }

    if dest_path.is_dir() {
        remove_directory(dest_path)
    } else if dest_path.exists() {
        remove_file(dest_path)
    } else {
        remove_symlink(dest_path) 
    }?;

    if let Some(path) = dest_path.parent() {
        if ! path.exists() { 
            create_directory(&path)?;
        }
    }

    soft_link(&src_path, dest_path)
}

pub fn create_hard_link(src: &str, dest: &str) -> Result<(), String> {   
    let src_path = Path::new(&src); 
    let dest_path = Path::new(&dest); 

    if ! src_path.exists() {
        Err(format!("Source file '{}': entity not found.", &src))?
    }

    if ! dest_path.exists() {
        if let Some(path) = dest_path.parent() {
            if ! path.exists() {
                remove_symlink(&path)?;
                create_directory(&path)?;    
            }
        }

        remove_symlink(dest_path)?;
        hard_link(src_path, dest_path)
    } else {
        let meta_dest = metadata(&dest_path)?;
        let meta_src = metadata(&src_path)?; 

        if meta_src.ino() != meta_dest.ino() {
            if meta_dest.is_dir() {
                remove_directory(dest_path)
            } else {
                remove_file(dest_path)
            }?;

            hard_link(src_path, dest_path)?;
        }

        Ok(())
    }
}

fn queue_status(queue: &HashSet<&str>, compare: &str, max_chars: usize) -> String {
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
            diff = queue.len()-strs.len();
            break;
        }

        strs.push(contrast);
    }

    for idx in 0..strs.len() {
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
        string.push_str("Synchronizing containers..");
    }

    string
}

fn metadata(path: &Path) -> Result<Metadata,String> {
    match fs::metadata(path) {
        Ok(meta) => Ok(meta),
        Err(err) => Err(format!("Failed to obtain metadata for '{}': {}", path.to_str().unwrap(), err.kind())),
    }
}

fn hard_link(src_path: &Path, dest_path: &Path) -> Result<(),String> {
    if let Err(err) = fs::hard_link(src_path,dest_path) {
           Err(format!("Failed to link '{}': {}", dest_path.to_str().unwrap(), err.kind()))?
    }

    Ok(())
}

fn soft_link<'a>(src_path: &'a Path, dest_path: &'a Path) -> Result<(),String> {
    if let Err(err) = symlink(src_path, dest_path) {
        Err(format!("Failed to create symlink '{}': {}", dest_path.to_str().unwrap(), err.kind()))? 
    }

    Ok(())
}

fn create_directory(path: &Path) -> Result<(), String> {
    if let Err(err) = fs::create_dir_all(path) {
        Err(format!("Failed to create directory tree '{}': {}", path.to_str().unwrap(), err.kind()))?
    }

    Ok(())
} 

fn remove_directory(path: &Path) -> Result<(), String> {
    if let Err(err) = fs::remove_dir_all(path) {
        Err(format!("Failed to delete directory tree '{}': {}", path.to_str().unwrap(), err.kind()))?
    }

    Ok(())
} 

fn remove_file(path: &Path) -> Result<(), String> {
    if let Err(err) = fs::remove_file(path) { 
        Err(format!("Failed to remove file '{}': {}", path.to_str().unwrap(), err.kind()))?
    }

    Ok(())
}

fn remove_symlink(path: &Path) -> Result<(),String> {
    if let Ok(_) = fs::read_link(path) {
        if let Err(err) = fs::remove_file(path) {         
            Err(format!("Failed to delete symlink '{}': {}", path.to_str().unwrap(), err.kind()))?
        } 
    }

    Ok(())
}
