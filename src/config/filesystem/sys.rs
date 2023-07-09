use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SYSFS {
    #[serde(skip_serializing_if = "is_default_path", default = "default_path")] 
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for SYSFS {
    fn check(&self, vars: &InsVars) -> Result<(),Error> {  
        for dir in self.path.iter() {
            if ! Path::new(&format!("/sys/{}",dir)).exists() {
                Err(Error::new("SYSFS", format!("/sys/{} is inaccessible.", dir), true))?
            }
        }
        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) { 
        for dir in self.path.iter() { 
            args.robind(format!("/sys/{}", dir), format!("/sys/{}", dir));
        }
    }
}

fn is_default_path(path: &Vec<String>) -> bool {
    if path == &default_path() { return true; } false
}

fn default_path() -> Vec<String> {
    let vec: Vec<String> = ["block".into(), 
                            "bus".into(), 
                            "class".into(), 
                            "dev".into(), 
                            "devices".into()].to_vec(); vec
}