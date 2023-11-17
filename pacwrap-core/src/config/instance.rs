#![allow(unused_variables)]

use std::fmt::Display;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::config::permission::{Permission, none::NONE};
use crate::config::filesystem::{Filesystem, root::ROOT, home::HOME};
use crate::config::dbus::Dbus;
use crate::config::vars::InsVars;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instance {
    #[serde(flatten)]  
    metadata: InstanceMetadata,
    #[serde(flatten)]
    runtime: InstanceRuntime,
}

impl Instance { 
    pub fn new(ctype: InstanceType, pkg: Vec<Rc<str>>, deps: Vec<Rc<str>>) -> Self {
        Self {
            metadata: InstanceMetadata::new(ctype,pkg,deps),
            runtime: InstanceRuntime::new(), 
        }
    }
}

#[derive(Clone)]
pub struct InstanceHandle {
    instance: Instance,
    vars: InsVars,
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

    pub fn config(&self) -> &InstanceRuntime {
        &self.instance.runtime
    }

    pub fn metadata_mut(&mut self) -> &mut InstanceMetadata {
        &mut self.instance.metadata
    }

    pub fn metadata(&self) -> &InstanceMetadata {
        &self.instance.metadata
    }

    pub fn vars(&self) -> &InsVars {
        &self.vars
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceRuntime {
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

impl InstanceRuntime {
    pub fn new() -> Self {
        let default_fs: [Box<dyn Filesystem>; 2]  = [Box::new(ROOT {}), Box::new(HOME {})];  
        let default_per: [Box<dyn Permission>; 1]  = [Box::new(NONE {})]; 
        let fs: Vec<Box<dyn Filesystem>> = Vec::from(default_fs);
        let per: Vec<Box<dyn Permission>> = Vec::from(default_per); 
 
        Self {
            allow_forking: false,
            retain_session: false,
            enable_userns: false,
            permissions: per,
            dbus: Vec::new(),
            filesystems: fs,
        }
    }

    pub fn permissions(&self) -> &Vec<Box<dyn Permission>> { &self.permissions }
    pub fn filesystem(&self) -> &Vec<Box<dyn Filesystem>> { &self.filesystems }
    pub fn dbus(&self) -> &Vec<Box<dyn Dbus>> { &self.dbus } 
    pub fn allow_forking(&self) -> &bool { &self.allow_forking }
    pub fn enable_userns(&self) -> &bool { &self.enable_userns } 
    pub fn retain_session(&self) -> &bool { &self.retain_session }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum InstanceType {
    LINK,
    BASE,
    DEP,
    ROOT
}

#[allow(dead_code)]
impl InstanceType {
    pub fn new(instype: &str) -> Self {
        match instype {
            "BASE" => Self::BASE,
            "ROOT" => Self::ROOT,
            "DEP" => Self::DEP,
            "LINK" => Self::LINK,
            _ => Self::BASE
        }
    }

    fn as_str<'a>(&self) -> &'a str {
        match self {
            Self::ROOT => "ROOT",
            Self::LINK => "LINK",
            Self::BASE => "BASE",
            Self::DEP => "DEP"
        }
    }
}

impl Display for InstanceType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        fmt.write_str(self.as_str())
    }
}

impl Default for InstanceType {
    fn default() -> Self {
        Self::BASE
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceMetadata {
    #[serde(default)]  
    container_type: InstanceType, 
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    dependencies: Vec<Rc<str>>,    
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    explicit_packages: Vec<Rc<str>>,
    #[serde(default = "time_as_seconds")]
    meta_version: u64,
}

impl InstanceMetadata {
    fn new(ctype: InstanceType, pkg: Vec<Rc<str>>, deps: Vec<Rc<str>>) -> Self {
        Self {
            container_type: ctype,
            dependencies: deps,
            explicit_packages: pkg, 
            meta_version: time_as_seconds(),
        }
    }

    pub fn set(&mut self, dep: Vec<Rc<str>>, pkg: Vec<Rc<str>>) {
            self.dependencies=dep;
            self.explicit_packages=pkg;
            self.meta_version = time_as_seconds(); 

    }

    pub fn container_type(&self) -> &InstanceType { 
        &self.container_type 
    }

    pub fn dependencies(&self) -> &Vec<Rc<str>> { 
        &self.dependencies 
    }
    
    pub fn explicit_packages(&self) -> &Vec<Rc<str>> { 
        &self.explicit_packages 
    }
}

fn time_as_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
