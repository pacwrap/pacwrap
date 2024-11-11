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
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        filesystem::{BindError, Filesystem, Mount, Permission},
        ContainerVariables,
    },
    err,
    exec::args::ExecutionArgs,
    Error,
    Result,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToRoot {
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "volumes")]
    mounts: Vec<Mount>,
}

#[typetag::serde(name = "to_root")]
impl Filesystem for ToRoot {
    fn qualify(&self, _vars: &ContainerVariables) -> Result<()> {
        if self.mounts.is_empty() {
            err!(BindError::Warn("Mount volumes undeclared.".into()))?
        }

        for m in self.mounts.iter() {
            if m.path.is_empty() {
                err!(BindError::Warn("Mount volumes undeclared.".into()))?
            }

            check_mount(&m.path)?
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _vars: &ContainerVariables) {
        for m in self.mounts.iter() {
            bind_filesystem(args, &m.permission, &m.path, &m.dest);
        }
    }

    fn module(&self) -> &'static str {
        "to_root"
    }
}

fn bind_filesystem(args: &mut ExecutionArgs, permission: &Permission, src: &str, dest: &str) {
    let dest = match dest.is_empty() {
        true => src,
        false => dest,
    };

    args.bind(permission, src, dest);
}

fn check_mount(path: &String) -> Result<()> {
    if !Path::new(path).exists() {
        err!(BindError::Fail("Source path not found.".into()))?
    }

    Ok(())
}
