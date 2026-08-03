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

use std::{fs::read_dir, path::Path, result::Result as StdResult};

use indexmap::IndexMap;

use crate::{
    ErrorKind,
    PathContext,
    config::{ConfigError, ContainerHandle, ContainerType, provide_handle, provide_new_handle},
    constants::{CONFIG_DIR, CONTAINER_DIR},
    eprintln_warn,
    error::*,
};

use super::{ContainerVariables, handle};

pub struct ContainerCache<'a> {
    instances: IndexMap<&'a str, ContainerHandle<'a>>,
}

impl Default for ContainerCache<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ContainerCache<'a> {
    pub fn new() -> Self {
        Self {
            instances: IndexMap::new(),
        }
    }

    pub fn add(&mut self, ins: &'a str, instype: ContainerType, deps: Vec<&'a str>) -> Result<()> {
        if self.instances.get(ins).is_some() {
            Err(ConfigError::AlreadyExists(ins.into()))?
        }

        for dep in deps.iter() {
            if self.instances.get(dep).is_none() {
                Err(ErrorKind::DependencyNotFound((*dep).into(), ins.into()))?
            }
        }

        self.register(ins, provide_new_handle(ins, instype, deps.to_vec())?);
        Ok(())
    }

    pub fn replace(&mut self, ins: &'a str, handle: ContainerHandle<'a>) -> Result<()> {
        self.register(ins, handle.default_vars());
        Ok(())
    }

    pub fn add_handle(&mut self, ins: &'a str, handle: ContainerHandle<'a>) -> Result<()> {
        if self.instances.get(ins).is_some() {
            Err(ConfigError::AlreadyExists(ins.into()))?
        }

        self.register(ins, handle.default_vars());
        Ok(())
    }

    fn map(&mut self, ins: &'a str) -> Result<()> {
        if self.instances.get(ins).is_some() {
            Err(ConfigError::AlreadyExists(ins.to_owned()))?
        }

        self.register(
            ins,
            match provide_handle(ins) {
                Ok(ins) => ins,
                Err(error) => {
                    eprintln_warn!("{error}");
                    return Ok(());
                }
            },
        );
        Ok(())
    }

    fn register(&mut self, ins: &'a str, handle: ContainerHandle<'a>) {
        self.instances.insert(ins, handle);
    }

    pub fn registered(&self) -> Vec<&'a str> {
        self.instances.iter().map(|a| *a.0).collect()
    }

    pub fn registered_handles(&'a self) -> Vec<&'a ContainerHandle<'a>> {
        self.instances.iter().map(|a| a.1).collect()
    }

    pub fn filter_target(&'_ self, target: &[&'_ str], filter: Vec<ContainerType>) -> Vec<&'_ str> {
        self.instances
            .iter()
            .filter(|a| target.contains(a.0) && (filter.contains(a.1.metadata().container_type()) || filter.is_empty()))
            .map(|a| *a.0)
            .collect()
    }

    pub fn filter_target_handle(&'_ self, target: &[&'_ str], filter: Vec<ContainerType>) -> Vec<&'_ ContainerHandle<'_>> {
        self.instances
            .iter()
            .filter(|a| target.contains(a.0) && (filter.contains(a.1.metadata().container_type()) || filter.is_empty()))
            .map(|a| a.1)
            .collect()
    }

    pub fn count(&self, filter: Vec<ContainerType>) -> usize {
        self.instances.iter().filter(|a| filter.contains(a.1.metadata().container_type())).count()
    }

    pub fn filter(&self, filter: Vec<ContainerType>) -> Vec<&'_ str> {
        self.instances
            .iter()
            .filter(|a| filter.contains(a.1.metadata().container_type()))
            .map(|a| *a.0)
            .collect()
    }

    pub fn filter_handle(&'_ self, filter: Vec<ContainerType>) -> Vec<&'_ ContainerHandle<'_>> {
        self.instances
            .iter()
            .filter(|a| filter.contains(a.1.metadata().container_type()))
            .map(|a| a.1)
            .collect()
    }

    pub fn obtain_base_handle(&'_ self) -> Option<&'_ ContainerHandle<'_>> {
        self.filter_handle(vec![ContainerType::Base])
            .iter()
            .find(|a| Path::new(a.vars().root()).exists())
            .copied()
    }

    pub fn get_instance(&'_ self, ins: &'_ str) -> Result<&'_ ContainerHandle<'_>> {
        match self.instances.get(ins) {
            Some(ins) => Ok(ins),
            None => Err(ErrorKind::InstanceNotFound(ins.into()))?,
        }
    }

    pub fn get_instance_option(&'_ self, ins: &'_ str) -> Option<&'a ContainerHandle<'_>> {
        self.instances.get(ins)
    }
}

pub fn populate_from<'a>(vec: &[&'a str]) -> Result<ContainerCache<'a>> {
    let mut cache = ContainerCache::new();

    for name in vec {
        cache.map(name)?;
    }

    Ok(cache)
}

pub fn populate_config_from<'a>(vec: &Vec<&'a str>) -> Result<ContainerCache<'a>> {
    let mut cache = ContainerCache::new();

    for name in vec {
        cache.add_handle(name, handle(ContainerVariables::new(name))?)?;
    }

    Ok(cache)
}

pub fn populate<'a>() -> Result<ContainerCache<'a>> {
    populate_from(
        &read_dir(*CONTAINER_DIR)
            .context_path(*CONTAINER_DIR)?
            .filter_map(StdResult::ok)
            .filter(|e| e.metadata().is_ok_and(|f| f.is_dir() || f.is_symlink()))
            .filter_map(|e| e.file_name().to_str().map(|f| f.to_string().leak() as &'a str))
            .collect::<Vec<&str>>(),
    )
}

pub fn populate_config<'a>() -> Result<ContainerCache<'a>> {
    let path = format!("{}/container", *CONFIG_DIR);

    populate_config_from(
        &read_dir(&path)
            .context_path(path)?
            .filter_map(StdResult::ok)
            .filter(|e| e.metadata().is_ok_and(|f| f.is_file() && !f.is_symlink()))
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|f| f.ends_with(".yml").then(|| f.to_string().leak().split_at(f.len() - 4).0))
            })
            .collect(),
    )
}
