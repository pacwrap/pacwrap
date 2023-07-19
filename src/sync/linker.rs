use std::fs::{create_dir, self, File, remove_file};
use std::os::unix::fs::symlink;
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, self, Receiver};
use std::thread::JoinHandle;

use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::collections::HashMap;
use console::Term;

use crate::config::{InstanceHandle, InstanceCache};
use crate::constants::LOCATION;
use crate::utils::{print_warning, print_error};

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

    fn files(&mut self) -> &mut IndexMap<String, (String,bool,bool)> {
        &mut self.files
    }
}

pub struct Linker {
    hlds: HashMap<String, HardLinkDS>, 
    hlds_res: HashMap<String, HardLinkDS>,
    linked: Vec<String>,
    progress: ProgressBar,
    writer_tr: Vec<JoinHandle<()>>
}

impl Linker {
    pub fn new() -> Self {
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
            writer_tr: Vec::new()
       }
    }

    pub fn start(&mut self, length: usize) {
        self.progress.set_draw_target(ProgressDrawTarget::stdout());
        self.progress.set_message("Synhcronizing containers..");
        self.progress.set_position(0);
        self.progress.set_length(progress_u64(length));
    }

    pub fn finish(&mut self) {
        self.progress.set_message("Synchronization complete.");
        self.progress.finish();
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

    pub fn link(&mut self, cache: &InstanceCache, containers: &Vec<String>, mut cached_threads: Vec<JoinHandle<()>>) -> Vec<JoinHandle<()>> { 
        let mut threads: Vec<_> = Vec::new(); 
        let (tx, rx): (Sender<(String, HardLinkDS)>, Receiver<(String, HardLinkDS)>) = mpsc::channel();

        for ins in containers.iter() { 
            if self.linked.contains(ins) {
                continue;
            }

            let inshandle = match cache.instances().get(ins) {
                Some(ins) => ins,
                None => {
                    print_error(format!("Linker: {} not found.", ins));
                    std::process::exit(1) 
                }
            };
          
            cached_threads = self.link(cache, inshandle.instance().dependencies(), cached_threads);

            if let Some(thread) = self.link_instance(inshandle, cache, tx.clone()) {
                threads.push(thread);
            }
            
            self.linked.push(ins.clone());
        }
    
        for thread in threads { 
            thread.join().unwrap();
            let recv = rx.recv().unwrap();
            if let None = self.hlds.get(&recv.0) {
                if recv.1.files.len() == 0 {
                    continue
                }
                self.hlds.insert(recv.0.clone(), recv.1.clone());
        
                if let Some(thread) = self.write_cache(recv.1, recv.0.clone()) {
                    cached_threads.push(thread);
                } 
            }
            self.progress.set_message(recv.0);
            self.progress.inc(1);
        }

        cached_threads
    }

    fn write_cache(&mut self, ds: HardLinkDS, dep: String) -> Option<JoinHandle<()>> {
        let output = File::create(format!("{}/hlds/{}.dat", LOCATION.get_data(), &dep)).unwrap();
        let thread = std::thread::Builder::new().name(format!("WRITER")).spawn(move ||{
    
        ciborium::into_writer(&ds, output).unwrap();
        }).unwrap();
        Some(thread)
    }
   
    fn link_instance(&mut self, inshandle: &InstanceHandle, cache: &InstanceCache, tx: Sender<(String, HardLinkDS)>) -> Option<JoinHandle<()>> {
        let deps = inshandle.instance().dependencies();
        let dep_depth = deps.len();
   
        if dep_depth == 0 { 
            return None;
        }
        
        let mut map = IndexMap::new();
        let dephandle = cache.instances().get(&deps[dep_depth-1]).unwrap();
        let dep = dephandle.vars().instance().clone();  
   
        for dep in deps {
            let dephandle = cache.instances().get(dep).unwrap();
            let ds = match self.hlds.get(dep) { 
                Some(ds) => ds.clone(),
                None => { HardLinkDS::new() }
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
            None => { HardLinkDS::new() }
        }; 

        let thread = std::thread::Builder::new().name(format!("PR-LINKER")).spawn(move ||{ 
            //println!("{} Linking {} against {}", style("->").cyan(), style(instance).bold(), style(&dep).bold()); 
            tx.send((dep, link_instance(ds, ds_res, root, map))).unwrap(); 
        }).unwrap();

        Some(thread)
    }    
}

fn link_instance(mut ds: HardLinkDS, ds_res: HardLinkDS, root: String, map: IndexMap<String,HardLinkDS>) -> HardLinkDS {
    if ds.files.len() == 0 {
        for hpds in map {
            let d = hpds.0;
            if hpds.1.files.len() == 0 { 
                let usr_path = format!("{}/usr", d);
                
                let entries = WalkDir::new(usr_path)
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in entries { 
                    let src = entry.path().to_str().unwrap().to_string();
                    let src_tr = src.split_at(d.len()).1.to_string();
                    
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

    for file in ds_res.files.iter() { 
        if let None = ds.files.get(file.0) {  
            let path = format!("{}{}", root, file.0);
            let file_path = Path::new(&path); 

            if ! file_path.exists() {
                if file.1.2 {
                    fs::remove_file(path).unwrap();
                }
            } else if file.1.1 && ! file.1.2 { 
                fs::remove_dir_all(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }

    for file in ds.files.iter() {
        let src_tr = file.0;
        let src = file.1.0.clone();
        let dest = format!("{}{}", root, src_tr);   

        if file.1.2 {
            create_soft_link(&src, &dest); 
        } else if file.1.1 {
            create_dir(&dest).ok();
        } else {
            create_hard_link(&src, &dest); 
        }
    }
    
    ds
}

fn create_soft_link(src: &str, dest: &str) {   
    let dest_path = Path::new(&dest);

    if ! dest_path.exists() {
        let attr = fs::read_link(src).unwrap();

        if let Ok(attr_dest) = fs::read_link(dest_path) {
            if attr.file_name().unwrap() == attr_dest.file_name().unwrap() {
                return;
            }
        }

        if let Ok(_) = remove_soft_link(dest_path, dest) {
            soft_link(&attr, dest_path);
        }
    } else {
        let attr = fs::read_link(src).unwrap();
        if let Ok(attr_dest) = fs::read_link(dest_path) {
            if attr.file_name().unwrap() != attr_dest.file_name().unwrap() {
                if let Ok(_) = remove_soft_link(dest_path, dest) {
                    soft_link(&attr, dest_path);
                }
            }
        }
    }
}

fn remove_soft_link(dest_path: &Path, dest: &str) -> Result<(),()> {
    if let Ok(_) = fs::read_link(dest_path) {
        if let Err(_) = fs::remove_file(dest_path) {        
            print_warning(format!("'{}': Failed to delete symlink.", dest));
            Err(())? 
        } 
    }
    Ok(())
}

fn soft_link<'a>(src_path: impl Into<&'a PathBuf>, dest_path: &'a Path) {
    if let Err(_) = symlink(src_path.into(), dest_path) {
        print_warning(format!("'{}': Failed to create symlink", dest_path.to_str().unwrap())); 
    }
}

pub fn create_hard_link(src_path: &str, dest_path: &str) {   
    if Path::new(&dest_path).exists() {
        let meta_dest = fs::metadata(&dest_path).unwrap();
       let meta_src = fs::metadata(&src_path).unwrap(); 

        if meta_src.ino() != meta_dest.ino() {
            if let Ok(_) = remove_file(&dest_path) {
                hard_link(src_path, dest_path);
            }
        }
    } else {
        hard_link(src_path, dest_path);
    }
}

fn hard_link(src_path: &str, dest_path: &str) {
    if let Err(err) = fs::hard_link(src_path,dest_path) {
        print_warning(format!("Failed to link '{}': {}", dest_path, err.kind()));
    } 
 
}

pub fn wait_on(writer_tr: Vec<JoinHandle<()>>) {
    for thread in writer_tr {
        thread.join().unwrap();
    }
}

fn progress_u64(u: usize) -> u64 {
    match u.try_into() { Ok(i) => i, Err(_) => 0 }
}
