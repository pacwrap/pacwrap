use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::os::unix::prelude::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Sender, self, Receiver};

use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::collections::HashMap;
use console::{Term, style};

use crate::config::{InstanceHandle, InstanceCache};
use crate::constants::LOCATION;
use crate::utils::{print_warning, print_error};

#[derive(Debug)]
enum Error {
    ThreadPoolUninitialised
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FilesystemState {
    files: IndexMap<Arc<str>, (Arc<str>, bool, bool)>
}

impl FilesystemState {
    fn new() -> Self {
        Self {
            files: IndexMap::new()
        }
    }
}

pub struct FilesystemStateSync<'a> {
    state_map: HashMap<Arc<str>, FilesystemState>, 
    state_map_prev: HashMap<Arc<str>, FilesystemState>,
    linked: Vec<Rc<str>>,
    progress: ProgressBar,
    cache: &'a InstanceCache,
    pool: Option<ThreadPool>
}

impl <'a>FilesystemStateSync<'a> {
    pub fn new(inscache: &'a InstanceCache) -> Self {
        let size = Term::size(&Term::stdout());
        let width = (size.1 / 2).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();
        let style = ProgressStyle::with_template(&(width_str+"} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[">", "✓"]); 
        let pr = ProgressBar::new(0).with_style(style);
        
        pr.set_draw_target(ProgressDrawTarget::hidden());

        Self {
            pool: None,
            progress: pr,
            state_map: HashMap::new(),
            state_map_prev: HashMap::new(),
            linked: Vec::new(),
            cache: inscache,
        }
    }

    pub fn engage(&mut self, containers: &Vec<Rc<str>>) {
        let (tx, rx) = self.link(containers, mpsc::channel()); 
        
        drop(tx); 
        
        while let Ok(()) = rx.recv() {}    
    }
  
    fn link(&mut self, containers: &Vec<Rc<str>>, mut write_chan: (Sender<()>, Receiver<()>)) -> (Sender<()>, Receiver<()>) { 
        let (tx, rx): (Sender<(Arc<str>, FilesystemState)>, Receiver<(Arc<str>, FilesystemState)>) = mpsc::channel();

        for ins in containers.iter() { 
            if self.linked.contains(ins) {
                continue;
            }

            let inshandle = match self.cache.instances().get(ins) {
                Some(ins) => ins,
                None => {
                    print_error(format!("Linker: {} not found.", ins));
                    std::process::exit(1) 
                }
            };
          
            write_chan = self.link(inshandle.metadata().dependencies(), write_chan);
            self.link_instance(inshandle, tx.clone());
            self.linked.push(ins.clone());
        }

        drop(tx);
        self.wait(rx, &write_chan);
        write_chan
    }

    fn wait(&mut self, rx: Receiver<(Arc<str>, FilesystemState)>, write_chan: &(Sender<()>, Receiver<()>)) {
        while let Ok(recv) = rx.recv() {

            if let None = self.state_map.get(&recv.0) {
                if recv.1.files.len() == 0 {
                    continue
                }

                self.state_map.insert(recv.0.clone(), recv.1.clone());
                self.write(write_chan.0.clone(), recv.1, recv.0.clone());
            }

            self.progress.set_message(recv.0.to_string());
            self.progress.inc(1);
        } 
    }

    fn load(&mut self, instance: &Arc<str>) -> Option<FilesystemState> {
        let path_string = format!("{}/hlds/{}.dat", LOCATION.get_data(), instance);
        let file = match File::open(path_string) { 
            Ok(file) => file, Err(_) => return Some(FilesystemState::new())
        };

        match ciborium::from_reader(file) {
            Ok(st) => self.state_map_prev.insert(instance.clone(), st),
            Err(err) => { 
                print_error(format!("Loading filesystem state data from {}: {:?}", instance, err)); 
                Some(FilesystemState::new())  
            }
        }
    }

    fn write(&mut self, tx: Sender<()>, ds: FilesystemState, dep: Arc<str>) {
        let output = File::create(format!("{}/hlds/{}.dat", LOCATION.get_data(), &dep)).unwrap();
        self.pool().unwrap().spawn(move ||{ 
            ciborium::into_writer(&ds, output).unwrap();
            drop(tx);
        });
    } 

    fn link_instance(&mut self, inshandle: &InstanceHandle, tx: Sender<(Arc<str>, FilesystemState)>) {
        let deps = inshandle.metadata().dependencies();
        let dep_depth = deps.len();
   
        if dep_depth == 0 { 
            return;
        }
        
        let mut map = Vec::new();
        let dephandle = self.cache.instances().get(&deps[dep_depth-1]).unwrap();
        let dep = Arc::from(dephandle.vars().instance().as_ref()); 
        let root = inshandle.vars().root().clone();
        let ds_res = match self.state_map_prev.get(&dep) { 
            Some(ds) => ds.clone(),
            None => match self.load(&dep) {
                Some(new) => new,
                None => self.state_map_prev.get(&dep).unwrap().clone()
            }
        }; 
        let ds = match self.state_map.get(&dep) { 
            Some(ds) => ds.clone(),
            None => FilesystemState::new()
        }; 

        for dep in deps {
            let dephandle = self.cache.instances().get(dep).unwrap();
            let ds = match self.state_map.get(&Arc::from(dep.as_ref())) { 
                Some(ds) => ds.clone(),
                None => FilesystemState::new()
            };
            map.push((dephandle.vars().root().clone(), ds));
        }

        self.pool().unwrap().spawn(move ||{
            let state = filesystem_state(ds, map);

            link_filesystem(&state, &root);
            delete_files(&state, &ds_res, &root);
            delete_directories(&state, &ds_res, &root);
            tx.send((dep, state)).unwrap(); 
        })
    }

    fn pool(&self) -> Result<&ThreadPool, Error> {
        match self.pool.as_ref() {
          Some(pool) =>  Ok(pool),
          None => Err(Error::ThreadPoolUninitialised)
        }
    }

    pub fn prepare_single(&mut self) {
        println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing container filesystem...")));     

        if let None = self.pool {
            self.pool = Some(ThreadPoolBuilder::new()
                .thread_name(|f| { format!("PW-LINKER-{}", f) })
                .num_threads(2)
                .build()
                .unwrap());  
        }
    }

    pub fn prepare(&mut self, length: usize) {
        println!("{} {} ",style("::").bold().green(), style("Synchronizing container filesystems...").bold());  

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

    pub fn release(self) -> Option<FilesystemStateSync<'a>> {
        drop(self);
        None
    }
}

fn filesystem_state(mut state: FilesystemState, map: Vec<(Arc<str>,FilesystemState)>) -> FilesystemState {
    if state.files.len() == 0 {
        for ins_state in map {
            if ins_state.1.files.len() == 0 {
                let entries = WalkDir::new(format!("{}/usr", ins_state.0))
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in entries { 
                    let src: Arc<str> = entry.path().to_str().unwrap().into();
                    let src_tr: Arc<str> = src.split_at(ins_state.0.len()).1.into();
                    
                    if let Some(_) = state.files.get(&src_tr) {
                        continue
                    }

                    let metadata = entry.metadata().unwrap(); 
                    
                    state.files.insert(src_tr, (src,metadata.is_dir(),metadata.is_symlink()));
                }
             } else {
                state.files.extend(ins_state.1.files);
             }
        }
    }

    state
}

fn link_filesystem(state: &FilesystemState, root: &str) {
    state.files.par_iter().for_each(|file| {
        if file.1.2 {
            if let Err(error) = create_soft_link(&file.1.0, &format!("{}{}", root, file.0)) {
                print_warning(error);
            } 
        } else if ! file.1.1 {
            if let Err(error) = create_hard_link(&file.1.0, &format!("{}{}", root, file.0)) {
                print_warning(error);
            }
        }
    });
}
 
fn delete_files(state: &FilesystemState, state_res: &FilesystemState, root: &str) { 
    let (tx, rx) = mpsc::sync_channel(0);
    let tx_clone: mpsc::SyncSender<()> = tx.clone();

    state_res.files.par_iter().for_each(|file| { 
        if let None = state.files.get(file.0) {  
            let _ = tx_clone; 
            let dest: &str = &format!("{}{}", root, file.0);
            let path = Path::new(dest); 

            if ! path.exists() {
                if file.1.2 {
                    remove_soft_link(&path).ok();
                }
                return;
            }

            if file.1.2 {
                if let Err(error) = remove_soft_link(path) {
                    print_warning(error);
                } 
            } else if ! file.1.1 {
                if let Err(error) = remove_file(path) {
                    print_warning(error);
                }
            }
        }
    });

    drop(tx);
    rx.try_iter();
}

fn delete_directories(state: &FilesystemState, state_res: &FilesystemState, root: &str) { 
    state_res.files.par_iter().for_each(move |file| { 
        if let None = state.files.get(file.0) {  
           let dest: &str = &format!("{}{}", root, file.0);
            let path = Path::new(dest); 

            if path.exists() && file.1.1 { 
                if let Err(error) = remove_directory(path) {
                    print_warning(error);
                }
            }
        }
    });
}

fn create_soft_link(src: &str, dest: &str) -> Result<(),String> {   
    let dest_path = Path::new(&dest);

    if ! dest_path.exists() {
        let src_path = fs::read_link(src).unwrap();

        if let Ok(src_path_dest) = fs::read_link(dest_path) {
            if src_path.file_name().unwrap() == src_path_dest.file_name().unwrap() {
                return Ok(());
            }
        }

        let result = remove_soft_link(dest_path); 

        match result {
            Err(_) => result,
            Ok(_) => {
                if let Some(path) = dest_path.parent() {
                    if ! path.exists() { 
                        let result = create_directory(&path);

                        if let Err(_) = result {
                            result?
                        } 
                    }
                }

                soft_link(&src_path, dest_path)
            }
        }
    } else { 
        if let Ok(attr_dest) = fs::read_link(dest_path) {
            let attr = fs::read_link(src).unwrap();
        
            if attr.file_name().unwrap() == attr_dest.file_name().unwrap() {
                return Ok(());
            }
            
            let result = remove_soft_link(dest_path);

            match result {
                Err(_) => result, 
                Ok(_) => soft_link(&attr, dest_path)
            }?
        }
   
        Ok(())
    }
}

pub fn create_hard_link(src: &str, dest: &str) -> Result<(), String> {   
    let src_path = Path::new(&src); 
    let dest_path = Path::new(&dest); 
    
    if ! dest_path.exists() {
        if ! src_path.exists() {
            Err(format!("Source file '{}': entity not found.", &dest))?
        }

        if let Some(path) = dest_path.parent() {
            if ! path.exists() {
                let result = create_directory(&path);
                
                if let Err(_) = result {
                    result?
                }
            }
        }

        hard_link(src_path, dest_path)
   } else {
        if ! src_path.exists() {
            Err(format!("Source file '{}': entity not found.", &dest))?
        }

        let meta_dest = fs::metadata(&dest_path).unwrap();
        let meta_src = fs::metadata(&src_path).unwrap(); 

        if meta_src.ino() != meta_dest.ino() {
             let result = remove_file(dest_path);

            match result {
                Err(_) => result, 
                Ok(_) => hard_link(src_path, dest_path)
            }?
        }

        Ok(())
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

fn remove_soft_link(path: &Path) -> Result<(),String> {
    if let Ok(_) = fs::read_link(path) {
        if let Err(err) = fs::remove_file(path) {         
            Err(format!("Failed to delete symlink '{}': {}", path.to_str().unwrap(), err.kind()))?
        } 
    }

    Ok(())
}
