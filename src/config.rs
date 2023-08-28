use std::io::Read;
use std::io::Write;
use std::vec::Vec;
use std::path::Path;
use std::fs::File;
use std::process::exit;

use crate::utils::print_error;

pub use crate::config::filesystem::Filesystem;
pub use crate::config::permission::Permission;
pub use crate::config::dbus::Dbus;

pub use cache::InstanceCache;
pub use instance::{InstanceHandle, Instance, InstanceType};
pub use vars::InsVars;

pub mod vars;
pub mod filesystem;
pub mod permission;
pub mod dbus;
pub mod cache;
pub mod instance;

pub fn save_handle(ins: &InstanceHandle) -> Result<(), String> {   
    let mut f = match File::create(Path::new(ins.vars().config_path().as_ref())) {
        Ok(f) => f,
        Err(error) => Err(format!("{}", error))?
    };
    let config = config_to_string(ins.instance());
    
    match write!(f, "{}", config) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("{}", error))
    }
}

pub fn provide_some_handle(instance: &str) -> Option<InstanceHandle> {
    let vars = InsVars::new(instance); 
    let path: &str = vars.config_path().as_ref();
        
    match File::open(path) {
        Ok(file) => {
            let str = read_into_string(file);
            let config = read_config(str.as_str());
            
            Some(InstanceHandle::new(config, vars))
        },
        Err(_) => None
    }
}

pub fn provide_handle(instance: &str) -> InstanceHandle {
    let vars = InsVars::new(instance); 
    let path: &str = vars.config_path().as_ref();

    match File::open(path) {
        Ok(file) => {
            let str = read_into_string(file);
            let config = read_config(str.as_str());

            InstanceHandle::new(config, vars)
        },
        Err(_) => {
            let config = Instance::new(InstanceType::BASE, Vec::new(), Vec::new());
            
            InstanceHandle::new(config, vars) 
        }
    }
}

fn read_into_string(mut file: File) -> String {
    let mut config: String = String::new();
    match file.read_to_string(&mut config) {
        Ok(_) => config,
        Err(error) => { 
            print_error(format!("{}", error));
            exit(2);
        },
    }
}

fn config_to_string(cfg: &Instance) -> String {
    match serde_yaml::to_string(cfg) {
        Ok(file) => file,
        Err(error) => { 
            print_error(format!("{}", error));
            exit(2);
        }
    }
}

fn read_config(str: &str) -> Instance {
    match serde_yaml::from_str(str) {
        Ok(file) => return file,
        Err(error) => { 
            print_error(format!("{}", error));
            exit(2);
        }
    }
}
