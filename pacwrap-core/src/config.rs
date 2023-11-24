use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::fs::File;

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
pub mod init;

pub fn save_handle(ins: &InstanceHandle) -> Result<(), String> {   
    let mut f = match File::create(Path::new(ins.vars().config_path())) {
        Ok(f) => f,
        Err(error) => Err(format!("{}", error))?
    };
    let config = match serde_yaml::to_string(&ins.instance()) {
        Ok(file) => file,
        Err(error) => Err(format!("{}", error))?,
    };
    
    match write!(f, "{}", config) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("{}", error))
    }
}

#[inline]
pub fn provide_handle(instance: &str) -> Result<InstanceHandle, String> { 
    let vars = InsVars::new(instance);

    if ! Path::new(vars.root()).exists() {  
        Err(format!("Container '{instance}' doesn't exist."))?
    }

    handle(instance, vars)
}

#[inline]
pub fn provide_new_handle(instance: &str) -> Result<InstanceHandle, String> {
    handle(instance, InsVars::new(instance))
}

fn handle<'a>(instance: &str, vars: InsVars<'a>) -> Result<InstanceHandle<'a>, String> {
    match File::open(vars.config_path()) {
        Ok(file) => {
            let config = match serde_yaml::from_reader(&file) {
                Ok(file) => file,
                Err(error) => Err(format!("'{instance}.yml': {error}"))?
            };

            Ok(InstanceHandle::new(config, vars))
        },
        Err(error) => match error.kind() {
            ErrorKind::NotFound => Err(format!("Configuration file for container '{instance}' is missing.")),
            _ => Err(format!("'{}': {error}", vars.config_path())),
        } 
    }
}
