#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use std::vec::Vec;
use std::env::var;
use std::path::Path;
use std::fs::File;
use std::process::exit;
use std::collections::HashMap;

use crate::config::filesystem::Filesystem;
use crate::config::permission::Permission;
use crate::config::permission::none::NONE;
use crate::config::filesystem::root::ROOT;
use crate::config::filesystem::home::HOME;
use crate::utils::print_error;
use crate::config::dbus::Dbus;
use crate::Arguments;

pub use vars::InsVars;
pub mod vars;
pub mod filesystem;
pub mod permission;
pub mod dbus;

#[derive(Serialize, Deserialize)]
pub struct Instance {
    #[serde(default)]  
    container_type: String, 
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    dependencies: Vec<String>,    
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    explicit_packages: Vec<String>, 
    #[serde(default)]  
    enable_userns: bool, 
    #[serde(default)]  
    retain_session: bool,     
    #[serde(default)]  
    allow_forking: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    filesystems: Vec<Box<dyn Filesystem>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    permissions: Vec<Box<dyn Permission>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    dbus: Vec<Box<dyn Dbus>>, 

}

impl Instance { 
    pub fn new(ctype: String, pkg: Vec<String>, deps: Vec<String>) -> Self {
        let mut fs: Vec<Box<dyn Filesystem>> = Vec::new();
        let mut per: Vec<Box<dyn Permission>> = Vec::new(); 

        fs.push(Box::new(ROOT {}));
        fs.push(Box::new(HOME {}));
        per.push(Box::new(NONE {}));

        Self {
            container_type: ctype,
            dependencies: deps,
            explicit_packages: pkg,
            allow_forking: false,
            retain_session: false,
            enable_userns: false,
            permissions: per,
            dbus: Vec::new(),
            filesystems: fs,
        }
     }

    fn set(&mut self, ctype: String, dep: Vec<String>, pkg: Vec<String>) {
          self.container_type=ctype;
          self.dependencies=dep;
          self.explicit_packages=pkg;
    }

    pub fn container_type(&self) -> &String {&self.container_type}
    pub fn permissions(&self) -> &Vec<Box<dyn Permission>> { &self.permissions }
    pub fn filesystem(&self) -> &Vec<Box<dyn Filesystem>> { &self.filesystems }
    pub fn dbus(&self) -> &Vec<Box<dyn Dbus>> { &self.dbus }
    pub fn allow_forking(&self) -> &bool { &self.allow_forking }
    pub fn enable_userns(&self) -> &bool { &self.enable_userns } 
    pub fn retain_session(&self) -> &bool { &self.retain_session }
    pub fn dependencies(&self) -> &Vec<String> { &self.dependencies }
    pub fn explicit_packages(&self) -> &Vec<String> { &self.explicit_packages }
}


pub fn aux_config() {
    let args = Arguments::new(1, "-", HashMap::from([("--save".into(), "s".into()),
                                                     ("--load".into(),"l".into())]));
    let instance = &args.get_targets()[0];

    match args.get_switch().as_str() {
        s if s.contains("s") => save_bash_configuration(instance),
        s if s.contains("l") => bash_configuration(&instance),
        &_ => print_error(format!("Invalid switch sequence.")), 
    }       
}

pub fn save_bash_configuration(ins: &String) {
    let pkgs: Vec<String> = var("PACWRAP_PKGS").unwrap().split_whitespace().map(str::to_string).collect(); 
    let deps: Vec<String> = var("PACWRAP_DEPS").unwrap().split_whitespace().map(str::to_string).collect();
    let ctype = var("PACWRAP_TYPE").unwrap(); 
    let vars = InsVars::new(ins);
    let mut instance: Instance;
    let path: &str = vars.config_path().as_str();

    match File::open(path) {
        Ok(file) => instance = read_yaml(file),
        Err(_) => instance = Instance::new(ctype.clone(), pkgs.clone(), deps.clone()),
    }
    
    instance.set(ctype, deps, pkgs);
    save_configuration(&instance, path.to_string()); 
}

pub fn bash_configuration(instance: &String) {
    let vars = InsVars::new(instance);
    let ins = &load_configuration(vars.config_path());
    let depends = ins.dependencies();
    let pkgs = ins.explicit_packages();
    let mut pkgs_string = String::new();
    let mut depends_string = String::new();

    println!("INSTANCE_CONFIG[{},0]={}", instance, ins.container_type());   

    for i in depends.iter() {
        depends_string.push_str(&format!("{} ", i));    
    }
    println!("INSTANCE_CONFIG[{},1]=\"{}\"", instance, depends_string);

    for i in pkgs.iter() {
        pkgs_string.push_str(&format!("{} ", i));
    }
    println!("INSTANCE_CONFIG[{},3]=\"{}\"", instance, pkgs_string);
}

pub fn save_configuration(ins: &Instance, config_path: String) {
    let f = File::create(Path::new(&config_path)).expect("Couldn't open file");
    serde_yaml::to_writer(f, &ins).unwrap();
}


fn read_yaml(file: File) -> Instance {
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
