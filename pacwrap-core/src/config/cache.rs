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

use std::{fs::read_dir, path::Path};

use indexmap::IndexMap;

use crate::{
    config::{provide_handle, provide_new_handle, ConfigError, ContainerHandle, ContainerType},
    constants::{CONFIG_DIR, CONTAINER_DIR},
    err,
    error::*,
    ErrorKind,
};

use super::{handle, ContainerVariables};

pub struct ContainerCache<'a> {
    instances: IndexMap<&'a str, ContainerHandle<'a>>,
}

impl<'a> ContainerCache<'a> {
    pub fn new() -> Self {
        Self {
            instances: IndexMap::new(),
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

        Ok(self.register(ins, provide_new_handle(ins, instype, deps.iter().map(|a| (*a).into()).collect())?))
    }

    pub fn replace(&mut self, ins: &'a str, handle: ContainerHandle<'a>) -> Result<()> {
        Ok(self.register(ins, handle.default_vars()))
    }

    pub fn add_handle(&mut self, ins: &'a str, handle: ContainerHandle<'a>) -> Result<()> {
        if let Some(_) = self.instances.get(ins) {
            err!(ConfigError::AlreadyExists(ins.into()))?
        }

        Ok(self.register(ins, handle.default_vars()))
    }

    fn map(&mut self, ins: &'a str) -> Result<()> {
        if let Some(_) = self.instances.get(ins) {
            err!(ConfigError::AlreadyExists(ins.to_owned()))?
        }

        Ok(self.register(
            ins,
            match provide_handle(ins) {
                Ok(ins) => ins,
                Err(error) => {
                    error.warn();
                    return Ok(());
                }
            },
        ))
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

    pub fn filter_target(&'a self, target: &Vec<&'a str>, filter: Vec<ContainerType>) -> Vec<&'a str> {
        self.instances
            .iter()
            .filter(|a| target.contains(a.0) && (filter.contains(a.1.metadata().container_type()) || filter.is_empty()))
            .map(|a| *a.0)
            .collect()
    }

    pub fn filter_target_handle(&'a self, target: &Vec<&'a str>, filter: Vec<ContainerType>) -> Vec<&'a ContainerHandle<'a>> {
        self.instances
            .iter()
            .filter(|a| target.contains(a.0) && (filter.contains(a.1.metadata().container_type()) || filter.is_empty()))
            .map(|a| a.1)
            .collect()
    }

    pub fn count(&self, filter: Vec<ContainerType>) -> usize {
        self.instances.iter().filter(|a| filter.contains(a.1.metadata().container_type())).count()
    }

    pub fn filter(&self, filter: Vec<ContainerType>) -> Vec<&'a str> {
        self.instances
            .iter()
            .filter(|a| filter.contains(a.1.metadata().container_type()))
            .map(|a| *a.0)
            .collect()
    }

    pub fn filter_handle(&'a self, filter: Vec<ContainerType>) -> Vec<&'a ContainerHandle<'a>> {
        self.instances
            .iter()
            .filter(|a| filter.contains(a.1.metadata().container_type()))
            .map(|a| a.1)
            .collect()
    }

    pub fn obtain_base_handle(&self) -> Option<&ContainerHandle> {
        self.filter_handle(vec![ContainerType::Base])
            .iter()
            .find(|a| Path::new(a.vars().root()).exists())
            .copied()
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

pub fn populate_config_from<'a>(vec: &Vec<&'a str>) -> Result<ContainerCache<'a>> {
    let mut cache = ContainerCache::new();

    for name in vec {
        cache.add_handle(&name, handle(ContainerVariables::new(name))?)?;
    }

    Ok(cache)
}

pub fn populate<'a>() -> Result<ContainerCache<'a>> {
    populate_from(
        &read_dir(*CONTAINER_DIR)
            .prepend_io(|| CONTAINER_DIR.to_string())?
            .filter(|e| e.as_ref().is_ok_and(|e| e.metadata().is_ok_and(|m| m.is_dir() | m.is_symlink())))
            .filter_map(|e| match e.unwrap().file_name().to_str() {
                Some(str) => Some(str.to_string().leak() as &'a str),
                None => None,
            })
            .collect(),
    )
}

pub fn populate_config<'a>() -> Result<ContainerCache<'a>> {
    populate_config_from(
        &read_dir(&format!("{}/container", *CONFIG_DIR))
            .prepend_io(|| format!("{}/container", *CONFIG_DIR))?
            .filter(|e| e.as_ref().is_ok_and(|e| e.metadata().is_ok_and(|m| m.is_file() && !m.is_symlink())))
            .filter_map(|e| match e.unwrap().file_name().to_str() {
                Some(f) => f.ends_with(".yml").then(|| f.to_string().leak().split_at(f.len() - 4).0),
                None => None,
            })
            .collect(),
    )
}
