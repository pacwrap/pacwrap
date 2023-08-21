use std::collections::HashMap;
use std::fs::read_dir;

use crate::constants::LOCATION;
use crate::config::{self, InstanceHandle};

use super::instance::InstanceType;


pub struct InstanceCache {
    instances: HashMap<String,InstanceHandle>,
    registered: Vec<String>,
    containers_base: Vec<String>,
    containers_dep: Vec<String>,
    containers_root: Vec<String>
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

    pub fn populate_from(&mut self, containers: &Vec<String>, recursion: bool) {
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
            for f in dir {
                if let Ok(file) = f {
                    let name: String = file.file_name()
                        .to_str()
                        .unwrap()
                        .to_string();

                    if self.map(&name) {      
                        self.registered.push(name);
                    }
                }
            }
        }
    }

    fn map(&mut self, ins: &String) -> bool {
        let mut register = true;
        if let None = self.instances.get(ins) {
            let config = config::provide_handle(ins);
           
            match config.metadata().container_type() {
                InstanceType::BASE => self.containers_base.push(ins.clone()),
                InstanceType::DEP => self.containers_dep.push(ins.clone()),
                InstanceType::ROOT => self.containers_root.push(ins.clone()),
                InstanceType::LINK => register = false,
            } 

            self.instances.insert(ins.clone(), config);
        }
        return register;
    }

    pub fn registered(&self) -> &Vec<String> { &self.registered }
    pub fn containers_base(&self) -> &Vec<String> { &self.containers_base }
    pub fn containers_dep(&self) -> &Vec<String> { &self.containers_dep }
    pub fn containers_root(&self) -> &Vec<String> { &self.containers_root }
    pub fn instances(&self) -> &HashMap<String,InstanceHandle> { &self.instances }
}
