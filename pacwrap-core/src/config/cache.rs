use std::collections::HashMap;
use std::fs::read_dir;

use crate::constants::LOCATION;
use crate::config::{self, InstanceHandle};
use crate::utils::print_warning;

use super::instance::InstanceType;

pub struct InstanceCache<'a> {
    instances: HashMap<&'a str,InstanceHandle<'a>>,
    registered: Vec<&'a str>,
    registered_base: Vec<&'a str>,
    registered_dep: Vec<&'a str>,
    registered_root: Vec<&'a str>
}

impl <'a>InstanceCache<'a> {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            registered: Vec::new(),
            registered_base: Vec::new(),
            registered_dep: Vec::new(),
            registered_root: Vec::new(),
        }
    }

    fn map(&mut self, ins: &'a str) -> bool {
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
                    InstanceType::BASE => self.registered_base.push(ins),
                    InstanceType::DEP => self.registered_dep.push(ins),
                    InstanceType::ROOT => self.registered_root.push(ins),
                    InstanceType::LINK => return false,
                } 

                self.instances.insert(ins, config);
                true
            }
        }
    }

    pub fn registered(&self) -> &Vec<&'a str> { 
        &self.registered 
    }
   
    pub fn registered_base(&self) -> &Vec<&'a str> { 
        &self.registered_base 
    }
    
    pub fn registered_dep(&self) -> &Vec<&'a str> { 
        &self.registered_dep 
    }

    pub fn registered_root(&self) -> &Vec<&'a str> { 
        &self.registered_root 
    }

    pub fn obtain_base_handle(&self) -> Option<&InstanceHandle> {
        match self.registered_base.get(0) {
            Some(instance) => self.instances.get(instance), None => None,
        }
    }

    pub fn get_instance(&self, ins: &str) -> Option<&InstanceHandle> { 
        self.instances.get(ins)
    }
}

pub fn populate<'a>() -> Result<InstanceCache<'a>, String> {
    let mut cache = InstanceCache::new();

    for name in roots()? {
        if cache.map(&name) {      
            cache.registered.push(name);      
        } 
    }

    Ok(cache)
}

fn roots<'a>() -> Result<Vec<&'a str>, String> { 
    match read_dir(format!("{}/root", LOCATION.get_data())) {
        Ok(dir) => Ok(dir.filter(|f| match f { 
            Ok(f) => match f.metadata() {
                Ok(meta) => meta.is_dir(), Err(_) => false, 
            }, 
            Err(_) => false })
        .map(|s| match s {
                Ok(f) => f.file_name()
                    .to_str()
                    .unwrap_or("")
                    .to_string()
                    .leak(), 
                Err(_) => "",
            })
        .filter_map(|e| match e.is_empty() {
                true => None,
                false => Some(e)
            })
        .collect()),
        Err(error) => Err(format!("'{}/root': {error}", LOCATION.get_data())),
    }
}
