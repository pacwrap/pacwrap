use std::collections::HashMap;
use std::fs::read_dir;
use std::rc::Rc;

use crate::constants::LOCATION;
use crate::config::{self, InstanceHandle};
use crate::utils::print_warning;

use super::instance::InstanceType;

pub struct InstanceCache {
    instances: HashMap<Rc<str>,InstanceHandle>,
    registered: Vec<Rc<str>>,
    containers_base: Vec<Rc<str>>,
    containers_dep: Vec<Rc<str>>,
    containers_root: Vec<Rc<str>>
}

impl InstanceCache {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            registered: Vec::new(),
            containers_base: Vec::new(),
            containers_dep: Vec::new(),
            containers_root: Vec::new(),
        }
    }

    pub fn populate_from(&mut self, containers: &Vec<Rc<str>>, recursion: bool) {
        for name in containers {
            if self.map(&name) {      
                self.registered.push(name.clone());
                let deps = self.instances.get(name)
                    .unwrap()
                    .metadata()
                    .dependencies()
                    .clone();

                if recursion {
                    self.populate_from(&deps, recursion); 
                }
            }
        }
    } 

    pub fn populate(&mut self) {
        if let Ok(dir) = read_dir(format!("{}/root", LOCATION.get_data())) {
            for file in dir.filter_map(|f| match f { Ok(f) => Some(f), Err(_) => None }) {
                let metadata = match file.metadata() {
                    Ok(metadata) => metadata, Err(_) => continue,
                };

                if ! metadata.is_dir() {
                    continue;
                }

                let name: Rc<str> = match file.file_name().to_str() {
                    Some(filename) => filename, None => continue,
                }.into();

                if self.map(&name) {      
                    self.registered.push(name);
                }
            }
        }
    }

    fn map(&mut self, ins: &Rc<str>) -> bool {
        match self.instances.get(ins) {
            Some(_) => false,
            None => {
                let config = match config::provide_handle(ins) {
                    Ok(ins) => ins, 
                    Err(error) => { 
                        print_warning(error); 
                        return false
                    }
                };

                match config.metadata().container_type() {
                    InstanceType::BASE => self.containers_base.push(ins.clone()),
                    InstanceType::DEP => self.containers_dep.push(ins.clone()),
                    InstanceType::ROOT => self.containers_root.push(ins.clone()),
                    InstanceType::LINK => return false,
                } 

                self.instances.insert(ins.clone(), config);
                true
            }
        }
    }

    pub fn registered(&self) -> &Vec<Rc<str>> { &self.registered }
    pub fn containers_base(&self) -> &Vec<Rc<str>> { &self.containers_base }
    pub fn containers_dep(&self) -> &Vec<Rc<str>> { &self.containers_dep }
    pub fn containers_root(&self) -> &Vec<Rc<str>> { &self.containers_root }
    pub fn instances(&self) -> &HashMap<Rc<str>,InstanceHandle> { &self.instances }
}
