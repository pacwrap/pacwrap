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

use std::{fmt::Display, 
    io::{Write, ErrorKind::NotFound}, 
    fmt::Formatter, 
    path::Path, 
    fs::File};

use serde::Serialize;

use crate::{err, impl_error, ErrorKind, error::*, constants::{BOLD, RESET, CONFIG_FILE}};

pub use self::{cache::InstanceCache, 
    instance::{Instance,
        InstanceHandle, 
        InstanceType}, 
    vars::InsVars, 
    filesystem::{Filesystem, BindError},
    permission::{Permission, PermError},
    dbus::Dbus,
    global::{Global, CONFIG}};

pub mod vars;
pub mod filesystem;
pub mod permission;
pub mod dbus;
pub mod cache;
pub mod instance;
pub mod init;
pub mod register;
mod global;

#[derive(Debug, Clone)]
pub enum ConfigError {
    Permission(&'static str, PermError),
    Filesystem(&'static str, BindError),
    Save(String, String),
    Load(String, String),
    AlreadyExists(String),
    ConfigNotFound(String),
}

impl_error!(ConfigError);

impl Display for ConfigError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::Filesystem(module, err) => write!(fmter, "Failed to register filesystem {}: {} ", module, err), 
            Self::Permission(module, err) => write!(fmter, "Failed to register permission {}: {} ", module, err),
            Self::Load(ins, error) => write!(fmter, "Failed to load '{ins}.yml': {error}"),
            Self::Save(ins, error) => write!(fmter, "Failed to save '{ins}.yml': {error}"),
            Self::AlreadyExists(ins) => write!(fmter, "Container {}{ins}{} already exists.", *BOLD, *RESET),
            Self::ConfigNotFound(ins) => write!(fmter, "Configuration '{}{ins}{}.yml' not found.", *BOLD, *RESET)
        }
    }
}

impl From<ConfigError> for String {
    fn from(value: ConfigError) -> Self {
        value.into()
    }
}

#[inline]
pub fn provide_handle(instance: &str) -> Result<InstanceHandle> { 
    let vars = InsVars::new(instance);

    if ! Path::new(vars.root()).exists() {  
        err!(ErrorKind::InstanceNotFound(instance.into()))?
    }

    handle(instance, vars)
}

#[inline]
pub fn provide_new_handle(instance: &str) -> Result<InstanceHandle> {
    handle(instance, InsVars::new(instance))
}

fn save<T: Serialize>(obj: &T, path: &str) -> Result<()> {   
    let mut f = match File::create(Path::new(path)) {
        Ok(f) => f,
        Err(error) => err!(ErrorKind::IOError(path.into(), error.kind()))?
    };
    let config = match serde_yaml::to_string(&obj) {
        Ok(file) => file,
        Err(error) => err!(ConfigError::Save(path.into(), error.to_string()))? 
    };
    
    match write!(f, "{}", config) {
        Ok(_) => Ok(()),
        Err(error) => err!(ErrorKind::IOError(path.into(), error.kind())) 
    }
}

fn handle<'a>(instance: &str, vars: InsVars<'a>) -> Result<InstanceHandle<'a>> {
    match File::open(vars.config_path()) {
        Ok(file) => {
            let config = match serde_yaml::from_reader(&file) {
                Ok(file) => file,
                Err(error) => err!(ConfigError::Load(vars.instance().into(), error.to_string()))?
            };

            Ok(InstanceHandle::new(config, vars))
        },
        Err(error) => match error.kind() {
            NotFound => err!(ConfigError::ConfigNotFound(instance.into()))?,
            _ => err!(ErrorKind::IOError(vars.config_path().into(), error.kind()))?,
        }
    }
}

fn config() -> Result<Global> {
    use std::io::ErrorKind::*;

    match File::open(*CONFIG_FILE) {
        Ok(file) => match serde_yaml::from_reader(&file) {
            Ok(file) => Ok(file),
            Err(error) => err!(ConfigError::Load(CONFIG_FILE.to_string(), error.to_string()))?
        },
        Err(error) => match error.kind() {
            NotFound => err!(ConfigError::ConfigNotFound(CONFIG_FILE.to_string()))?,
            _ => err!(ErrorKind::IOError(CONFIG_FILE.to_string(), error.kind()))?,
        }
    }
}
