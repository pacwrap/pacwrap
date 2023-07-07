use std::collections::HashMap;
use std::fs::read_dir;

use crate::constants::LOCATION;
use crate::config::{self, InsVars, InstanceHandle};


pub struct InstanceCache {
    instances: HashMap<String,InstanceHandle>,
    registered: Vec<String>,
    containers_base: Vec<String>,
    containers_dep: Vec<String>,
    containers_root: Vec<String>
}

impl InstanceCache {
    pub fn new() -> Self {
        let s = Self {
            instances: HashMap::new(),
            registered: Vec::new(),
            containers_base: Vec::new(),
            containers_dep: Vec::new(),
            containers_root: Vec::new(),
        };
        s.populate()
    }
 
    fn populate(mut self) -> Self {
        if let Ok(dir) = read_dir(format!("{}/root", LOCATION.get_data())) {
            for f in dir {
                if let Ok(file) = f {
                    let name: String = file.file_name().to_str().unwrap().to_string();
                    if self.map_instance(&name) {      
                        self.registered.push(name);
                    }
                }
            }
        }
        self
    }

    fn map_instance(&mut self, ins: &String) -> bool {
        let mut register = true;
        if let None = self.instances.get(ins) {
            let vars = InsVars::new(ins);
            let config_path = vars.config_path(); 
            let config = InstanceHandle::new(config::load_configuration(config_path), vars);
            
            if config.instance().container_type() == "BASE" {
                self.containers_base.push(ins.clone());
            } else if config.instance().container_type() == "DEP" {
                self.containers_dep.push(ins.clone());
            } else if config.instance().container_type() == "ROOT" {
                self.containers_root.push(ins.clone());
            } else {
               register = false; 
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
