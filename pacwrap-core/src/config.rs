/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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
    fmt::{Display, Formatter},
    fs::File,
    io::{ErrorKind::NotFound, Write},
    path::Path,
};

use serde::Serialize;

use crate::{
    constants::{BOLD, CONFIG_FILE, RESET},
    err,
    error::*,
    impl_error,
    ErrorKind,
};

pub use self::{
    cache::ContainerCache,
    container::{Container, ContainerHandle, ContainerType},
    dbus::Dbus,
    filesystem::{BindError, Filesystem},
    global::{global, Global},
    permission::{PermError, Permission},
    vars::ContainerVariables,
};

pub mod cache;
pub mod container;
pub mod dbus;
pub mod filesystem;
pub mod global;
pub mod init;
pub mod permission;
pub mod register;
pub mod vars;

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
            Self::Filesystem(module, err) => write!(fmter, "Failed to register filesystem '{}': {} ", module, err),
            Self::Permission(module, err) => write!(fmter, "Failed to register permission '{}': {} ", module, err),
            Self::Load(ins, error) => write!(fmter, "Failed to load '{ins}': {error}"),
            Self::Save(ins, error) => write!(fmter, "Failed to save '{ins}': {error}"),
            Self::AlreadyExists(ins) => write!(fmter, "Container '{}{ins}{}' already exists.", *BOLD, *RESET),
            Self::ConfigNotFound(path) => write!(fmter, "'{path}': Configuration not found."),
        }
    }
}

pub fn provide_handle(instance: &str) -> Result<ContainerHandle> {
    let vars = ContainerVariables::new(instance);

    if !Path::new(vars.root()).exists() {
        err!(ErrorKind::InstanceNotFound(instance.into()))?
    }

    handle(vars)
}

pub fn compose_handle<'a>(instance: &'a str, path: Option<&'a str>) -> Result<ContainerHandle<'a>> {
    let vars = match path {
        Some(path) => ContainerVariables::new(instance).config(path),
        None => ContainerVariables::new(instance),
    };

    if Path::new(vars.root()).exists() {
        err!(ConfigError::AlreadyExists(instance.into()))?
    }

    Ok(handle(vars)?.stamp().create())
}

pub fn provide_new_handle<'a>(instance: &'a str, instype: ContainerType, deps: Vec<&'a str>) -> Result<ContainerHandle<'a>> {
    match handle(ContainerVariables::new(instance)) {
        Ok(mut handle) => {
            handle.metadata_mut().set_metadata(deps, vec![]);
            Ok(handle.create())
        }
        Err(err) => {
            if let Ok(ConfigError::ConfigNotFound(..)) = err.downcast::<ConfigError>() {
                let cfg = Container::new(instype, deps, vec![]);
                let vars = ContainerVariables::new(instance);

                return Ok(ContainerHandle::new(cfg, vars).create());
            }

            Err(err)?
        }
    }
}

fn save<T: Serialize>(obj: &T, path: &str) -> Result<()> {
    let mut f = File::create(path).prepend_io(|| path.into())?;
    let config = match serde_yaml::to_string(&obj) {
        Ok(file) => file,
        Err(error) => err!(ConfigError::Save(path.into(), error.to_string()))?,
    };

    write!(f, "{}", config).prepend_io(|| path.into())
}

#[inline]
fn handle<'a>(vars: ContainerVariables) -> Result<ContainerHandle<'a>> {
    match File::open(vars.config_path()) {
        Ok(file) => {
            let config = match serde_yaml::from_reader(&file) {
                Ok(file) => file,
                Err(error) => err!(ConfigError::Load(vars.instance().into(), error.to_string()))?,
            };

            Ok(ContainerHandle::new(config, vars))
        }
        Err(error) => match error.kind() {
            NotFound => err!(ConfigError::ConfigNotFound(vars.config_path().into()))?,
            _ => err!(ErrorKind::IOError(vars.config_path().into(), error.kind()))?,
        },
    }
}

fn load_config() -> Result<Global> {
    match serde_yaml::from_reader(File::open(*CONFIG_FILE).prepend_io(|| CONFIG_FILE.to_string())?) {
        Ok(file) => Ok(file),
        Err(error) => err!(ConfigError::Load(CONFIG_FILE.to_string(), error.to_string()))?,
    }
}
