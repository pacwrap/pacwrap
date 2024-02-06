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

use crate::{
    config::{
        filesystem::{default_permission, is_default_permission, BindError, Filesystem},
        ContainerVariables,
    },
    constants::HOME,
    exec::args::ExecutionArgs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToHome {
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "volumes")]
    mounts: Vec<Mount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mount {
    #[serde(skip_serializing_if = "is_default_permission", default = "default_permission")]
    permission: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    path: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    dest: String,
}

#[typetag::serde(name = "to_home")]
impl Filesystem for ToHome {
    fn check(&self, _vars: &ContainerVariables) -> Result<(), BindError> {
        if self.mounts.len() == 0 {
            Err(BindError::Warn(format!("Mount volumes undeclared.")))?
        }

        for m in self.mounts.iter() {
            if m.path.len() == 0 {
                Err(BindError::Warn(format!("Mount volumes undeclared.")))?
            }

            if let Err(e) = check_mount(&m.permission, &m.path) {
                return Err(e);
            }
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &ContainerVariables) {
        for m in self.mounts.iter() {
            bind_filesystem(args, vars, &m.permission, &m.path, &m.dest);
        }
    }

    fn module(&self) -> &'static str {
        "to_home"
    }
}

fn bind_filesystem(args: &mut ExecutionArgs, vars: &ContainerVariables, permission: &str, src: &str, dest: &str) {
    let dest = match dest.is_empty() {
        false => dest,
        true => src,
    };
    let dest = &format!("{}/{}", vars.home_mount(), dest);
    let src = &format!("{}/{}", *HOME, src);

    match permission == "rw" {
        false => args.robind(src, dest),
        true => args.bind(src, dest),
    }
}

fn check_mount(permission: &String, path: &String) -> Result<(), BindError> {
    let per = permission.to_lowercase();

    if per != "ro" && per != "rw" {
        Err(BindError::Fail(format!("{} is an invalid permission.", permission)))?
    }

    if !Path::new(&format!("{}/{}", *HOME, &path)).exists() {
        Err(BindError::Fail(format!("~/{} not found.", path)))?
    }

    Ok(())
}
