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

use crate::{err, 
    ErrorKind,
    error::*,
    constants::DATA_DIR, 
    config::{self, InstanceHandle}};

use super::{InsVars, 
    ConfigError, 
    Instance, 
    instance::InstanceType};

pub struct InstanceCache<'a> {
    instances: HashMap<&'a str, InstanceHandle<'a>>,
    registered: Vec<&'a str>,
    registered_base: Vec<&'a str>,
    registered_dep: Vec<&'a str>,
    registered_root: Vec<&'a str>
}

impl <'a>InstanceCache<'a> {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            registered: Vec::new(),
            registered_base: Vec::new(),
            registered_dep: Vec::new(),
            registered_root: Vec::new(),
        }
    }

    pub fn add(&mut self, ins: &'a str, instype: InstanceType, deps: Vec<&'a str>) -> Result<()> {
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
                handle.metadata_mut().set(deps, vec!()); 
                handle 
            },
            Err(err) => match err.downcast::<ConfigError>() {
                Ok(error) => match error {
                    ConfigError::ConfigNotFound(_) => {
                        let vars = InsVars::new(ins);         
                        let cfg = Instance::new(instype, deps, vec!());
                        
                        InstanceHandle::new(cfg, vars)        
                    },
                    _ => Err(err)?,
                }
                _ => Err(err)?,
            },
        };

        Ok(self.register(ins, handle)) 
    }

    fn map(&mut self, ins: &'a str) -> Result<()>  {
        if let Some(_) = self.instances.get(ins) {
            err!(ConfigError::AlreadyExists(ins.to_owned()))?
        }

        Ok(self.register(ins, match config::provide_handle(ins) {
            Ok(ins) => ins, 
            Err(error) => { 
                error.warn();
                return Ok(())
            }
        }))
    }

    fn register(&mut self, ins: &'a str, handle: InstanceHandle<'a>) {
        match handle.metadata().container_type() {
            InstanceType::BASE => self.registered_base.push(ins),
            InstanceType::DEP => self.registered_dep.push(ins),
            InstanceType::ROOT => self.registered_root.push(ins),
            InstanceType::LINK => return,
        } 

        self.instances.insert(ins, handle);
        self.registered.push(ins);
    }

    pub fn registered(&self) -> &Vec<&'a str> { 
        &self.registered 
    }
   
    pub fn registered_base(&self) -> &Vec<&'a str> { 
        &self.registered_base 
    }
    
    pub fn registered_dep(&self) -> &Vec<&'a str> { 
        &self.registered_dep 
    }

    pub fn registered_root(&self) -> &Vec<&'a str> { 
        &self.registered_root 
    }

    pub fn obtain_base_handle(&self) -> Option<&InstanceHandle> {
        match self.registered_base.get(0) {
            Some(instance) => self.instances.get(instance), None => None,
        }
    }

    pub fn get_instance(&self, ins: &str) -> Result<&InstanceHandle> { 
        match self.instances.get(ins) {
            Some(ins) => Ok(ins), None => err!(ErrorKind::InstanceNotFound(ins.into())),
        }
    }

    pub fn get_instance_option(&self, ins: &str) -> Option<&InstanceHandle> { 
        self.instances.get(ins)
    }
}

pub fn populate_from<'a>(vec: &Vec<&'a str>) -> Result<InstanceCache<'a>> {
    let mut cache = InstanceCache::new();

    for name in vec {
        cache.map(&name)?;
    }

    Ok(cache)
}

pub fn populate<'a>() -> Result<InstanceCache<'a>> {
    populate_from(&roots()?)
}

fn roots<'a>() -> Result<Vec<&'a str>> { 
    match read_dir(format!("{}/root", *DATA_DIR)) {
        Ok(dir) => Ok(dir.filter(|f| match f {
            Ok(f) => match f.metadata() {
                Ok(meta) => meta.is_dir() | meta.is_symlink(), Err(_) => false 
            }, 
            Err(_) => false 
        })
        .map(|s| match s {
                Ok(f) => match f.file_name().to_str() {
                    Some(f) => f.to_owned().leak(), None => "", 
                },
                Err(_) => "",
            })
        .filter(|e| ! e.is_empty())
        .collect()),
        Err(error) => err!(ErrorKind::IOError(format!("{}/root", *DATA_DIR), error.kind())),
    }
}
