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

use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    result::Result as StdResult,
    vec::Vec,
};

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        dbus::Dbus,
        filesystem::{home::Home, root::Root, Filesystem},
        permission::{none::None, Permission},
        save,
        vars::ContainerVariables,
    },
    constants::UNIX_TIMESTAMP,
    Result,
};

#[derive(Deserialize, Serialize, Clone)]
#[serde(try_from = "ContainerShadow")]
pub struct Container<'a> {
    #[serde(flatten)]
    metadata: ContainerMetadata<'a>,
    #[serde(flatten)]
    runtime: ContainerRuntime,
}

#[derive(Deserialize, Clone)]
pub struct ContainerShadow<'a> {
    #[serde(flatten)]
    metadata: ContainerMetadata<'a>,
    #[serde(flatten)]
    runtime: ContainerRuntime,
}

impl<'a> TryFrom<ContainerShadow<'a>> for Container<'a> {
    type Error = &'static str;

    fn try_from(value: ContainerShadow<'a>) -> StdResult<Self, Self::Error> {
        if value.metadata.container_type == ContainerType::Base && value.metadata.dependencies.len() > 0 {
            Err("Dependencies cannot be specified for Base type containers.")?;
        }

        Ok(Self {
            metadata: value.metadata,
            runtime: value.runtime,
        })
    }
}

impl<'a> Container<'a> {
    pub fn new(ctype: ContainerType, deps: Vec<&'a str>, pkgs: Vec<&'a str>) -> Self {
        Self {
            metadata: ContainerMetadata::new(ctype, deps, pkgs),
            runtime: ContainerRuntime::new(),
        }
    }
}

#[derive(Clone)]
pub struct ContainerHandle<'a> {
    inner: Container<'a>,
    meta: ContainerVariables,
    creation: bool,
}

impl<'a> ContainerHandle<'a> {
    pub fn new(ins: Container<'a>, ins_vars: ContainerVariables) -> Self {
        Self {
            inner: ins,
            meta: ins_vars,
            creation: false,
        }
    }

    pub fn create(mut self) -> Self {
        self.creation = true;
        self
    }

    pub fn config(&self) -> &ContainerRuntime {
        &self.inner.runtime
    }

    pub fn metadata_mut(&mut self) -> &mut ContainerMetadata<'a> {
        &mut self.inner.metadata
    }

    pub fn metadata(&self) -> &ContainerMetadata {
        &self.inner.metadata
    }

    pub fn is_creation(&self) -> bool {
        self.creation
    }

    pub fn default_vars(mut self) -> Self {
        self.meta = ContainerVariables::new(self.meta.instance());
        self
    }

    pub fn vars(&self) -> &ContainerVariables {
        &self.meta
    }

    pub fn save(&self) -> Result<()> {
        save(&self.inner, self.meta.config_path())
    }
}

impl<'a> Debug for ContainerHandle<'a> {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmter, "{:?}", self.vars())?;
        write!(fmter, "{:?}", self.config())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContainerRuntime {
    #[serde(default)]
    enable_userns: bool,
    #[serde(default)]
    retain_session: bool,
    #[serde(default)]
    allow_forking: bool,
    #[serde(default = "default_true")]
    seccomp: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    filesystems: Vec<Box<dyn Filesystem>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    permissions: Vec<Box<dyn Permission>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    dbus: Vec<Box<dyn Dbus>>,
}

impl ContainerRuntime {
    pub fn new() -> Self {
        let default_fs: [Box<dyn Filesystem>; 2] = [Box::new(Root {}), Box::new(Home {})];
        let default_per: [Box<dyn Permission>; 1] = [Box::new(None {})];

        Self {
            seccomp: true,
            allow_forking: false,
            retain_session: false,
            enable_userns: false,
            permissions: Vec::from(default_per),
            dbus: Vec::new(),
            filesystems: Vec::from(default_fs),
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

impl Debug for ContainerRuntime {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(fmter, "allow_forking:       {}", self.allow_forking)?;
        writeln!(fmter, "retain_session:      {}", self.retain_session)?;
        writeln!(fmter, "enable_userns:       {}", self.enable_userns)?;
        writeln!(fmter, "seccomp:             {}", self.seccomp)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum ContainerType {
    Symbolic,
    Base,
    Slice,
    Aggregate,
}

impl ContainerType {
    fn as_str<'a>(&self) -> &'a str {
        match self {
            Self::Symbolic => "Sumbolic",
            Self::Base => "Base",
            Self::Slice => "Slice",
            Self::Aggregate => "Aggregate",
        }
    }
}

impl Display for ContainerType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        fmt.write_str(self.as_str())
    }
}

impl Default for ContainerType {
    fn default() -> Self {
        Self::Base
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContainerMetadata<'a> {
    #[serde(default)]
    container_type: ContainerType,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    dependencies: Vec<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    explicit_packages: Vec<Cow<'a, str>>,
    #[serde(default = "time_as_seconds")]
    meta_version: u64,
}

impl<'a> ContainerMetadata<'a> {
    fn new(ctype: ContainerType, deps: Vec<&'a str>, pkgs: Vec<&'a str>) -> Self {
        Self {
            container_type: ctype,
            dependencies: deps.iter().map(|a| (*a).into()).collect(),
            explicit_packages: pkgs.iter().map(|a| (*a).into()).collect(),
            meta_version: *UNIX_TIMESTAMP,
        }
    }

    pub fn set(&mut self, deps: Vec<&'a str>, pkgs: Vec<&'a str>) {
        self.dependencies = deps.iter().map(|a| (*a).into()).collect();
        self.explicit_packages = pkgs.iter().map(|a| (*a).into()).collect();
        self.meta_version = *UNIX_TIMESTAMP;
    }

    pub fn container_type(&self) -> &ContainerType {
        &self.container_type
    }

    pub fn dependencies(&'a self) -> Vec<&'a str> {
        self.dependencies.iter().map(|a| a.as_ref()).collect()
    }

    pub fn explicit_packages(&'a self) -> Vec<&'a str> {
        self.explicit_packages.iter().map(|a| a.as_ref()).collect()
    }

    pub fn timestamp(&self) -> u64 {
        self.meta_version
    }
}

fn default_true() -> bool {
    true
}

fn time_as_seconds() -> u64 {
    *UNIX_TIMESTAMP
}
