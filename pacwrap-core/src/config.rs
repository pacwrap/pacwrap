/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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
    Filesystem(&'static str, BindError),
    #[error("Failed to save '{0}': {1}")]
    Save(String, String),
    #[error("Failed to load '{0}': {1}")]
    Load(String, String),
    #[error("Container '{bold}{0}{reset}' already exists.", bold=*BOLD, reset=*RESET)]
    AlreadyExists(String),
    #[error("Configuration not found: {0}")]
    ConfigNotFound(std::io::Error),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl_error!(ConfigError);

pub fn provide_handle<'a>(instance: &str) -> Result<ContainerHandle<'a>> {
    let vars = ContainerVariables::new(instance);

    if !Path::new(vars.root()).exists() {
        Err(ErrorKind::InstanceNotFound(instance.into()))?
    }

    handle(vars)
}

pub fn compose_handle<'a>(instance: &'a str, path: Option<&'a str>) -> Result<ContainerHandle<'a>> {
    let vars = match path {
        Some(path) => ContainerVariables::new(instance).config(path),
        None => ContainerVariables::new(instance),
    };

    if Path::new(vars.root()).exists() {
        Err(ConfigError::AlreadyExists(instance.into()))?
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
            if let ErrorType::Config(ConfigError::ConfigNotFound(..)) = err.error {
                let cfg = Container::new(instype, deps, vec![]);
                let vars = ContainerVariables::new(instance);

                return Ok(ContainerHandle::new(cfg, vars).create());
            }

            Err(err)?
        }
    }
}

fn save<T: Serialize>(obj: &T, path: &str) -> Result<()> {
    let mut f = File::create(path).context_path(path)?;
    let config = serde_yaml::to_string(&obj).map_err(|err| ConfigError::Save(path.into(), err.to_string()))?;

    Ok(write!(f, "{}", config)?)
}

#[inline]
fn handle<'a>(vars: ContainerVariables) -> Result<ContainerHandle<'a>> {
    let file = File::open(vars.config_path()).context_path(vars.config_path());
    let file = match file {
        Ok(file) => file,
        Err(err) =>
            if let NotFound = err.kind() {
                Err(ConfigError::ConfigNotFound(err))?
            } else {
                Err(err)?
            },
    };
    let config = serde_yaml::from_reader(&file).map_err(|err| ConfigError::Load(vars.instance().into(), err.to_string()))?;

    Ok(ContainerHandle::new(config, vars))
}

fn load_config() -> Result<Global> {
    Ok(serde_yaml::from_reader(File::open(*CONFIG_FILE).context_path(*CONFIG_FILE)?)
        .map_err(|err| ConfigError::Load(CONFIG_FILE.to_string(), err.to_string()))?)
}
