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
    fs::File,
    io::{ErrorKind::NotFound, Write},
    path::Path,
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    ErrorKind,
    constants::{BOLD, CONFIG_FILE, RESET},
    err,
    error::*,
    impl_error,
};

pub use self::{
    cache::ContainerCache,
    container::{Container, ContainerHandle, ContainerType},
    dbus::Dbus,
    filesystem::{BindError, Filesystem},
    global::{Global, global},
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

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to register filesystem module '{0}': {1}'")]
    Permission(&'static str, PermError),
    #[error("Failed to register permission module '{0}': {1}")]
    Filesystem(&'static str, Error),
    #[error("Failed to save '{0}': {1}")]
    Save(String, String),
    #[error("Failed to load '{0}': {1}")]
    Load(String, String),
    #[error("Container '{bold}{0}{reset}' already exists.", bold=*BOLD, reset=*RESET)]
    AlreadyExists(String),
    #[error("'{0}': Configuration not found.")]
    ConfigNotFound(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl_error!(ConfigError);

impl From<&Error> for ConfigError {
    fn from(error: &Error) -> ConfigError {
        Self::InternalError(error.kind().to_string())
    }
}

pub fn provide_handle<'a>(instance: &str) -> Result<ContainerHandle<'a>> {
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
    let mut f = File::create(path).prepend_io(|| path)?;
    let config = match serde_yaml::to_string(&obj) {
        Ok(file) => file,
        Err(error) => err!(ConfigError::Save(path.into(), error.to_string()))?,
    };

    write!(f, "{}", config).prepend_io(|| path)
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
    match serde_yaml::from_reader(File::open(*CONFIG_FILE).prepend_io(|| *CONFIG_FILE)?) {
        Ok(file) => Ok(file),
        Err(error) => err!(ConfigError::Load(CONFIG_FILE.to_string(), error.to_string()))?,
    }
}
