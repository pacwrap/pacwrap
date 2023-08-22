use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::os::unix::prelude::MetadataExt;
use std::path::Path;
use std::sync::mpsc::{Sender, self, Receiver};

use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::collections::HashMap;
use console::Term;

use crate::config::{InstanceHandle, InstanceCache};
use crate::constants::LOCATION;
use crate::utils::{print_warning, print_error};
use super::utils::usize_into_u64;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HardLinkDS {
    files: IndexMap<String, (String,bool,bool)>
}

impl HardLinkDS {
    fn new() -> Self {
        Self {
            files: IndexMap::new()
        }
    }
}

pub struct Linker<'a> {
    hlds: HashMap<String, HardLinkDS>, 
    hlds_res: HashMap<String, HardLinkDS>,
    linked: Vec<String>,
    progress: ProgressBar,
    cache: &'a InstanceCache,
}

impl <'a>Linker<'a> {
    pub fn new(inscache: &'a InstanceCache) -> Self {
        let size = Term::size(&Term::stdout());
        let width = (size.1 / 2).to_string();
        let width_str = " {spinner:.green} {msg:<".to_owned()+width.as_str();
        let style = ProgressStyle::with_template(&(width_str+"} [{wide_bar}] {percent:<3}%"))
            .unwrap().progress_chars("#-").tick_strings(&[">", "✓"]); 
        let pr = ProgressBar::new(0).with_style(style);
        
        pr.set_draw_target(ProgressDrawTarget::hidden());

        Self {
            progress: pr,
            hlds: HashMap::new(),
            hlds_res: HashMap::new(),
            linked: Vec::new(),
            cache: inscache,
        }
    }

    fn build_pool(&mut self, workers: usize) -> ThreadPool {
        ThreadPoolBuilder::new()
                .num_threads(workers)
                .thread_name(|f| { format!("PW-LINKER-{}", f) })
                .build()
                .unwrap()
    }

    fn load_hlds(&mut self, instance: &String) -> Option<HardLinkDS> {
        let path_string = format!("{}/hlds/{}.dat", LOCATION.get_data(), instance);
        let file = match File::open(path_string) { 
            Ok(file) => file, Err(_) => return Some(HardLinkDS::new())
        };

        match ciborium::from_reader(file) {
            Ok(st) => self.hlds_res.insert(instance.clone(), st),
            Err(err) => { print_error(format!("load_hlds: {:?}", err)); Some(HardLinkDS::new())  }
        }
    }

    pub fn link(&mut self, containers: &Vec<String>, workers: usize) {
        let pool = self.build_pool(workers);
        let (tx, rx) = self.linker(containers, &pool, mpsc::channel());
        
        drop(tx);
        
        while let Ok(()) = rx.recv() {} 
    }
  
    fn linker(&mut self, containers: &Vec<String>, pool: &ThreadPool, mut write_chan: (Sender<()>, Receiver<()>)) -> (Sender<()>, Receiver<()>) { 
        let (tx, rx): (Sender<(String, HardLinkDS)>, Receiver<(String, HardLinkDS)>) = mpsc::channel();

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
          
            write_chan = self.linker(inshandle.metadata().dependencies(), pool, write_chan);
            self.link_instance(pool, inshandle, tx.clone());
            self.linked.push(ins.clone());
        }

        drop(tx);
        self.wait(pool, rx, &write_chan);
        write_chan
    }

    fn wait(&mut self, pool: &ThreadPool, rx: Receiver<(String, HardLinkDS)>, write_chan: &(Sender<()>, Receiver<()>)) {
        while let Ok(recv) = rx.recv() {
            if let None = self.hlds.get(&recv.0) {
                if recv.1.files.len() == 0 {
                    continue
                }

                self.hlds.insert(recv.0.clone(), recv.1.clone());
                self.write_cache(pool, write_chan.0.clone(), recv.1, recv.0.clone());
            }
            self.progress.set_message(recv.0);
            self.progress.inc(1);
        } 
    }

    fn write_cache(&mut self, pool: &ThreadPool, tx: Sender<()>, ds: HardLinkDS, dep: String) {
        let output = File::create(format!("{}/hlds/{}.dat", LOCATION.get_data(), &dep)).unwrap();
        pool.spawn(move ||{ 
            ciborium::into_writer(&ds, output).unwrap();
            drop(tx);
        });
    }
   
    fn link_instance(&mut self, pool: &ThreadPool, inshandle: &InstanceHandle, tx: Sender<(String, HardLinkDS)>) {
        let deps = inshandle.metadata().dependencies();
        let dep_depth = deps.len();
   
        if dep_depth == 0 { 
            return;
        }
        
        let mut map = IndexMap::new();
        let dephandle = self.cache.instances().get(&deps[dep_depth-1]).unwrap();
        let dep = dephandle.vars().instance().clone();  
   
        for dep in deps {
            let dephandle = self.cache.instances().get(dep).unwrap();
            let ds = match self.hlds.get(dep) { 
                Some(ds) => ds.clone(),
                None => HardLinkDS::new()
            };
            map.insert(dephandle.vars().root().clone(), ds);
        }


        let root = inshandle.vars().root().clone();
        let ds_res = match self.hlds_res.get(&dep) { 
            Some(ds) => ds.clone(),
            None => { 
                if let Some(new) = self.load_hlds(&dep) {
                     new
                } else {
                     self.hlds_res.get(&dep).unwrap().clone()
                }
            }
        }; 
        let ds = match self.hlds.get(&dep) { 
            Some(ds) => ds.clone(),
            None => HardLinkDS::new()
        }; 

        pool.spawn(move ||{ 
            tx.send((dep, link_instance(ds, ds_res, root, map))).unwrap(); 
        })
    }

    pub fn start(&mut self, length: usize) {
        self.progress.set_draw_target(ProgressDrawTarget::stdout());
        self.progress.set_message("Synhcronizing containers..");
        self.progress.set_position(0);
        self.progress.set_length(usize_into_u64(length));
    }

    pub fn set_cache(&mut self, inscache: &'a InstanceCache) {
        self.cache = inscache;
    }

    pub fn finish(&mut self) { 
        self.progress.set_message("Synchronization complete."); 
        self.progress.finish();
    }
}

fn link_instance(mut ds: HardLinkDS, ds_res: HardLinkDS, root: String, map: IndexMap<String,HardLinkDS>) -> HardLinkDS {
    if ds.files.len() == 0 {
        for hpds in map {
            if hpds.1.files.len() == 0 {
                let entries = WalkDir::new(format!("{}/usr", hpds.0))
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in entries { 
                    let src = entry.path().to_str().unwrap().to_string();
                    let src_tr = src.split_at(hpds.0.len()).1.to_string();
                    
                    if let Some(_) = ds.files.get(&src_tr) {
                        continue
                    }

                    let metadata = entry.metadata().unwrap(); 
                    
                    ds.files.insert(src_tr, (src,metadata.is_dir(),metadata.is_symlink()));
                }
             } else {
                ds.files.extend(hpds.1.files);
             }
        }
    }

    ds_res.files.par_iter().for_each(|file| { 
        if let None = ds.files.get(file.0) {  
            let dest = format!("{}{}", root, file.0);
            let path = Path::new(&dest); 

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
            } else if file.1.1 { 
                if let Err(error) = remove_directory(path) {
                    print_warning(error);
                }
            } else {
                if let Err(error) = remove_file(path) {
                    print_warning(error);
                }
            }
        }
    });

    ds.files.par_iter().for_each(|file| {
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
    
    ds
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
        let meta_dest = fs::metadata(&dest_path).unwrap();
        let meta_src = fs::metadata(&src_path).unwrap(); 

        if meta_src.ino() != meta_dest.ino() {
            if ! src_path.exists() {
                Err(format!("Source file '{}': entity not found.", &dest))?
            }

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
