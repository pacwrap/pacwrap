#![allow(unused_variables)]

use std::vec::Vec;
use std::path::Path;
use std::fs::File;
use std::process::exit;

use crate::utils::print_error;

pub use crate::config::filesystem::Filesystem;
pub use crate::config::permission::Permission;
pub use crate::config::dbus::Dbus;

pub use cache::InstanceCache;
pub use instance::{InstanceHandle, Instance};
pub use vars::InsVars;

pub mod vars;
pub mod filesystem;
pub mod permission;
pub mod dbus;
pub mod cache;
pub mod instance;

pub fn save_configuration(ins: &Instance, config_path: String) {
    let f = File::create(Path::new(&config_path)).expect("Couldn't open file");
    serde_yaml::to_writer(f, &ins).unwrap();
}


pub fn read_yaml(file: File) -> Instance {
    match serde_yaml::from_reader(file) {
        Ok(file) => return file,
        Err(error) => { 
            print_error(format!("{}", error));
            exit(2);    
        }
    }
}

pub fn load_configuration(config_path: &String) -> Instance {
    let path: &str = config_path.as_str();
    match File::open(path) {
        Ok(file) => read_yaml(file),
        Err(_) => Instance::new(format!("BASE"), Vec::new(), Vec::new()),
    }
}
