/*
 * pacwrap-core
 * 
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::borrow::Cow;
use std::fmt::{Display, Debug, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::config::permission::{Permission, none::NONE};
use crate::config::filesystem::{Filesystem, root::ROOT, home::HOME};
use crate::config::dbus::Dbus;
use crate::config::vars::InsVars;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instance<'a> {
    #[serde(flatten)]  
    metadata: InstanceMetadata<'a>,
    #[serde(flatten)]
    runtime: InstanceRuntime,
}

impl <'a>Instance<'a> { 
    pub fn new(ctype: InstanceType, deps: Vec<&'a str>, pkgs: Vec<&'a str>) -> Self {
        Self {
            metadata: InstanceMetadata::new(ctype,deps,pkgs),
            runtime: InstanceRuntime::new(), 
        }
    }
}

#[derive(Clone)]
pub struct InstanceHandle<'a> {
    instance: Instance<'a>,
    vars: InsVars<'a>,
}

impl <'a>InstanceHandle<'a> {
    pub fn new(ins: Instance<'a>, ins_vars: InsVars<'a>) -> Self {
        Self {
            instance: ins,
            vars: ins_vars,
        }
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn config(&self) -> &InstanceRuntime {
        &self.instance.runtime
    }

    pub fn metadata_mut(&mut self) -> &mut InstanceMetadata<'a> {
        &mut self.instance.metadata
    }

    pub fn metadata(&self) -> &InstanceMetadata {
        &self.instance.metadata
    }

    pub fn vars(&self) -> &InsVars {
        &self.vars
    }
}

impl <'a>Debug for InstanceHandle<'a> {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmter, "{:?}", self.vars())?;
        write!(fmter, "{:?}", self.config())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceRuntime {
    #[serde(default)]  
    enable_userns: bool, 
    #[serde(default)]  
    retain_session: bool,
    #[serde(default = "default_true")]
    seccomp: bool,
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
            seccomp: true, 
            allow_forking: false,
            retain_session: false,
            enable_userns: false,
            permissions: per,
            dbus: Vec::new(),
            filesystems: fs,
        }
    }

    pub fn permissions(&self) -> &Vec<Box<dyn Permission>> { 
        &self.permissions 
    }
    
    pub fn filesystem(&self) -> &Vec<Box<dyn Filesystem>> { 
        &self.filesystems 
    }
    
    pub fn dbus(&self) -> &Vec<Box<dyn Dbus>> { 
        &self.dbus 
    } 
    
    pub fn allow_forking(&self) -> &bool { 
        &self.allow_forking 
    }
    
    pub fn enable_userns(&self) -> &bool { 
        &self.enable_userns 
    } 
    
    pub fn retain_session(&self) -> &bool { 
        &self.retain_session 
    }

    pub fn seccomp(&self) -> &bool {
        &self.seccomp
    }
}

impl Debug for InstanceRuntime {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "allow_forking:       {}", self.allow_forking)?;
        writeln!(fmter, "retain_session:      {}", self.retain_session)?;
        writeln!(fmter, "enable_userns:       {}", self.enable_userns)?;
        writeln!(fmter, "seccomp:             {}", self.seccomp)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
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
pub struct InstanceMetadata<'a> {
    #[serde(default)]  
    container_type: InstanceType, 
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    dependencies: Vec<Cow<'a, str>>,    
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    explicit_packages: Vec<Cow<'a, str>>,
    #[serde(default = "time_as_seconds")]
    meta_version: u64,
}

impl <'a>InstanceMetadata<'a> {
    fn new(ctype: InstanceType, deps: Vec<&'a str>, pkgs: Vec<&'a str>) -> Self {
        Self {
            container_type: ctype,
            dependencies: deps.iter().map(|a| (*a).into()).collect(),
            explicit_packages: pkgs.iter().map(|a| (*a).into()).collect(),
            meta_version: time_as_seconds(),
        }
    }

    pub fn set(&mut self, deps: Vec<&'a str>, pkgs: Vec<&'a str>) {
        self.dependencies = deps.iter().map(|a| (*a).into()).collect();
        self.explicit_packages = pkgs.iter().map(|a| (*a).into()).collect();
        self.meta_version = time_as_seconds();
    }

    pub fn container_type(&self) -> &InstanceType { 
        &self.container_type 
    }

    pub fn dependencies(&'a self) -> Vec<&'a str> { 
        self.dependencies.iter().map(|a| a.as_ref()).collect()
    }
    
    pub fn explicit_packages(&'a self) -> Vec<&'a str> { 
        self.explicit_packages.iter().map(|a| a.as_ref()).collect()
    }
}

fn time_as_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn default_true() -> bool {
    true
}
