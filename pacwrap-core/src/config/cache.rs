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

use std::{collections::HashMap, fs::read_dir};

use crate::{
    config::{self, ConfigError, Container, ContainerHandle, ContainerType, ContainerVariables},
    constants::{CONTAINER_DIR, DATA_DIR},
    err,
    error::*,
    ErrorKind,
};

pub struct ContainerCache<'a> {
    instances: HashMap<&'a str, ContainerHandle<'a>>,
}

impl<'a> ContainerCache<'a> {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    pub fn add(&mut self, ins: &'a str, instype: ContainerType, deps: Vec<&'a str>) -> Result<()> {
        if let Some(_) = self.instances.get(ins) {
            err!(ConfigError::AlreadyExists(ins.into()))?
        }

        for dep in deps.iter() {
            if let None = self.instances.get(dep) {
                err!(ErrorKind::DependencyNotFound((*dep).into(), ins.into()))?
            }
        }

        let deps = deps.iter().map(|a| (*a).into()).collect();
        let handle = match config::provide_new_handle(ins) {
            Ok(mut handle) => {
                handle.metadata_mut().set(deps, vec![]);
                handle
            }
            Err(err) => match err.downcast::<ConfigError>() {
                Ok(error) => match error {
                    ConfigError::ConfigNotFound(_) => {
                        let vars = ContainerVariables::new(ins);
                        let cfg = Container::new(instype, deps, vec![]);

                        ContainerHandle::new(cfg, vars)
                    }
                    _ => Err(err)?,
                },
                _ => Err(err)?,
            },
        };

        Ok(self.register(ins, handle))
    }

    fn map(&mut self, ins: &'a str) -> Result<()> {
        if let Some(_) = self.instances.get(ins) {
            err!(ConfigError::AlreadyExists(ins.to_owned()))?
        }

        Ok(self.register(
            ins,
            match config::provide_handle(ins) {
                Ok(ins) => ins,
                Err(error) => {
                    error.warn();
                    return Ok(());
                }
            },
        ))
    }

    fn register(&mut self, ins: &'a str, handle: ContainerHandle<'a>) {
        if let ContainerType::Symbolic = handle.metadata().container_type() {
            return;
        }

        self.instances.insert(ins, handle);
    }

    pub fn registered(&self) -> Vec<&'a str> {
        self.instances.iter().map(|a| *a.0).collect()
    }

    pub fn filter(&self, filter: Vec<ContainerType>) -> Vec<&'a str> {
        self.instances
            .iter()
            .filter(|a| filter.contains(a.1.metadata().container_type()))
            .map(|a| *a.0)
            .collect()
    }

    pub fn obtain_base_handle(&self) -> Option<&ContainerHandle> {
        match self.filter(vec![ContainerType::Base]).get(0) {
            Some(instance) => self.instances.get(instance),
            None => None,
        }
    }

    pub fn get_instance(&self, ins: &str) -> Result<&ContainerHandle> {
        match self.instances.get(ins) {
            Some(ins) => Ok(ins),
            None => err!(ErrorKind::InstanceNotFound(ins.into())),
        }
    }

    pub fn get_instance_option(&self, ins: &str) -> Option<&ContainerHandle> {
        self.instances.get(ins)
    }
}

pub fn populate_from<'a>(vec: &Vec<&'a str>) -> Result<ContainerCache<'a>> {
    let mut cache = ContainerCache::new();

    for name in vec {
        cache.map(&name)?;
    }

    Ok(cache)
}

pub fn populate<'a>() -> Result<ContainerCache<'a>> {
    populate_from(&roots()?)
}

fn roots<'a>() -> Result<Vec<&'a str>> {
    match read_dir(*CONTAINER_DIR) {
        Ok(dir) => Ok(dir
            .filter(|f| match f {
                Ok(f) => match f.metadata() {
                    Ok(meta) => meta.is_dir() | meta.is_symlink(),
                    Err(_) => false,
                },
                Err(_) => false,
            })
            .map(|s| match s {
                Ok(f) => match f.file_name().to_str() {
                    Some(f) => f.to_owned().leak(),
                    None => "",
                },
                Err(_) => "",
            })
            .filter(|e| !e.is_empty())
            .collect()),
        Err(error) => err!(ErrorKind::IOError(format!("{}/root", *DATA_DIR), error.kind())),
    }
}
