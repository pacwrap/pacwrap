#![allow(unused_variables)]

use std::fmt::Display;
use std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::config::permission::{Permission, none::NONE};
use crate::config::filesystem::{Filesystem, root::ROOT, home::HOME};
use crate::config::dbus::Dbus;
use crate::config::vars::InsVars;

#[derive(Serialize, Deserialize)]
pub struct Instance {
    #[serde(flatten)]  
    metadata: InstanceMetadata,
    #[serde(flatten)]
    runtime: InstanceRuntime,
}

impl Instance { 
    pub fn new(ctype: InstanceType, pkg: Vec<String>, deps: Vec<String>) -> Self {
        Self {
            metadata: InstanceMetadata::new(ctype,pkg,deps),
            runtime: InstanceRuntime::new(), 
        }
    }
}

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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub enum InstanceType {
    LINK,
    BASE,
    DEP,
    ROOT
}

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

#[derive(Serialize, Deserialize)]
pub struct InstanceMetadata {
    #[serde(default)]  
    container_type: InstanceType, 
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    dependencies: Vec<String>,    
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    explicit_packages: Vec<String>,
}

impl InstanceMetadata {
    fn new(ctype: InstanceType, pkg: Vec<String>, deps: Vec<String>) -> Self {
        Self {
            container_type: ctype,
            dependencies: deps,
            explicit_packages: pkg, 
        }
    }

    pub fn set(&mut self, dep: Vec<String>, pkg: Vec<String>) {
          self.dependencies=dep;
          self.explicit_packages=pkg;
    }

    pub fn container_type(&self) -> &InstanceType { &self.container_type }
    pub fn dependencies(&self) -> &Vec<String> { &self.dependencies }
    pub fn explicit_packages(&self) -> &Vec<String> { &self.explicit_packages }
}
