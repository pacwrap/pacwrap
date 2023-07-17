#![allow(unused_variables)]

use std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::config::permission::none::NONE;
use crate::config::filesystem::root::ROOT;
use crate::config::filesystem::home::HOME;
use crate::config::filesystem::Filesystem;
use crate::config::permission::Permission;
use crate::config::dbus::Dbus;
use crate::config::vars::InsVars;

pub struct InstanceHandle {
    instance: Instance,
    vars: InsVars
}

impl InstanceHandle {
    pub fn new(ins: Instance, v: InsVars) -> Self {
        Self {
            instance: ins,
            vars: v,
        }
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn extract_instance(self) -> Instance {
        self.instance
    }

    pub fn vars(&self) -> &InsVars {
        &self.vars
    }
}

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
        let default_fs: [Box<dyn Filesystem>; 2]  = [Box::new(ROOT {}), Box::new(HOME {})];  
        let default_per: [Box<dyn Permission>; 1]  = [Box::new(NONE {})]; 
        let fs: Vec<Box<dyn Filesystem>> = Vec::from(default_fs);
        let per: Vec<Box<dyn Permission>> = Vec::from(default_per); 

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

    pub fn set(&mut self, ctype: String, dep: Vec<String>, pkg: Vec<String>) {
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
