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

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs, 
    config::InsVars, 
    config::filesystem::{Filesystem, BindError}};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SYSFS {
    #[serde(skip_serializing_if = "is_default_path", default = "default_path")] 
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for SYSFS {
    fn check(&self, _vars: &InsVars) -> Result<(), BindError> {  
        for dir in self.path.iter() {
            if ! Path::new(&format!("/sys/{}",dir)).exists() {
                Err(BindError::Fail(format!("/sys/{} is inaccessible.", dir)))?
            }
        }

        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, _: &InsVars) { 
        for dir in self.path.iter() { 
            args.robind(format!("/sys/{}", dir), format!("/sys/{}", dir));
        }
    }

    fn module(&self) -> &'static str {
        "SUSFS"
    }
}

fn is_default_path(path: &Vec<String>) -> bool {
    path == &default_path()
}

fn default_path() -> Vec<String> {
    vec!("block".into(), 
        "bus".into(), 
        "class".into(), 
        "dev".into(), 
        "devices".into())
}
