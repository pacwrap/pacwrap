use std::{collections::HashMap,
    io::ErrorKind::NotFound,
    fs::read_dir};

use crate::{ErrorKind, 
    constants::DATA_DIR, 
    config::{self, InstanceHandle}, 
    utils::print_warning};

use super::{InsVars, 
    ConfigError, 
    Instance, 
    instance::InstanceType};

pub struct InstanceCache<'a> {
    instances: HashMap<&'a str, InstanceHandle<'a>>,
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

    pub fn add(&mut self, ins: &'a str, instype: InstanceType, deps: Vec<&'a str>) -> Result<(),ErrorKind> {
        if let Some(_) = self.instances.get(ins) {
            Err(ErrorKind::Config(ConfigError::AlreadyExists(ins.into())))?
        }

        let deps = deps.iter().map(|a| (*a).into()).collect();          
        let handle = match config::provide_new_handle(ins) {
            Ok(mut handle) => { 
                handle.metadata_mut().set(deps, vec!()); 
                handle 
            },
            Err(error) => match error {          
                ErrorKind::IOError(_, kind) => match kind { 
                    NotFound => { 
                        let vars = InsVars::new(ins);         
                        let cfg = Instance::new(instype, deps, vec!());
                        
                        InstanceHandle::new(cfg, vars)
                    },
                    _ => Err(error)?
                },
                _ => Err(error)?
            }
        };

        Ok(self.register(ins, handle)) 
    }

    fn map(&mut self, ins: &'a str) -> Result<(),ErrorKind>  {
        if let Some(_) = self.instances.get(ins) {
            Err(ErrorKind::Config(ConfigError::AlreadyExists(ins.into())))?
        }

        Ok(self.register(ins, match config::provide_handle(ins) {
            Ok(ins) => ins, 
            Err(error) => { 
                print_warning(error.to_string()); 
                return Ok(())
            }
        }))
    }

    fn register(&mut self, ins: &'a str, handle: InstanceHandle<'a>) {
        match handle.metadata().container_type() {
            InstanceType::BASE => self.registered_base.push(ins),
            InstanceType::DEP => self.registered_dep.push(ins),
            InstanceType::ROOT => self.registered_root.push(ins),
            InstanceType::LINK => return,
        } 

        self.instances.insert(ins, handle);
        self.registered.push(ins);
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

pub fn populate_from<'a>(vec: &Vec<&'a str>) -> Result<InstanceCache<'a>, ErrorKind> {
    let mut cache = InstanceCache::new();

    for name in vec {
        cache.map(&name)?;
    }

    Ok(cache)
}

pub fn populate<'a>() -> Result<InstanceCache<'a>, ErrorKind> {
    populate_from(&roots()?)
}

fn roots<'a>() -> Result<Vec<&'a str>, ErrorKind> { 
    match read_dir(format!("{}/root", *DATA_DIR)) {
        Ok(dir) => Ok(dir.filter(|f| match f { 
            Ok(f) => match f.metadata() {
                Ok(meta) => meta.is_dir() | meta.is_symlink(), 
                Err(_) => false, 
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
        Err(error) => Err(ErrorKind::IOError(format!("'{}/root", *DATA_DIR), error.kind())),
    }
}
