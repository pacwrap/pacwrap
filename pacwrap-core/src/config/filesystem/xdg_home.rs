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
        filesystem::{
            BindError,
            Filesystem,
            Mount,
            Permission::{self, ReadOnly},
        },
        ContainerVariables,
    },
    constants::HOME,
    err,
    exec::args::ExecutionArgs,
    Error,
    Result,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdgHome {
    #[serde(skip_serializing_if = "Vec::is_empty", default = "xdg_default", rename = "volumes")]
    mounts: Vec<Mount>,
}

#[typetag::serde(name = "xdg_home")]
impl Filesystem for XdgHome {
    fn qualify(&self, _vars: &ContainerVariables) -> Result<()> {
        if self.mounts.is_empty() {
            err!(BindError::Warn("Mount volumes undeclared.".into()))?
        }

        for m in self.mounts.iter() {
            if m.path.is_empty() {
                err!(BindError::Warn("Mount volumes undeclared.".into()))?
            }

            check_mount(&m.path)?;
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _: &ContainerVariables) {
        let mounts = xdg_default();
        let mut mounts = mounts.iter().filter(|m| check_mount(&m.path).is_ok()).collect::<Vec<&Mount>>();

        mounts.extend(self.mounts.iter().filter(|a| !mounts.contains(a)).collect::<Vec<&Mount>>());

        for m in mounts {
            bind_filesystem(args, &m.permission, &m.path);
        }
    }

    fn module(&self) -> &'static str {
        "xdg_home"
    }
}

fn bind_filesystem(args: &mut ExecutionArgs, permission: &Permission, dest: &str) {
    let path = &format!("{}/{}", *HOME, dest);

    args.bind(permission, path, path);
}

fn check_mount(path: &str) -> Result<()> {
    if !Path::new(&format!("{}/{}", *HOME, &path)).exists() {
        err!(BindError::Fail(format!("~/{} not found.", path)))?
    }

    Ok(())
}

fn xdg_default() -> Vec<Mount> {
    ["Downloads", "Documents", "Pictures", "Videos", "Music"]
        .iter()
        .map(|d| Mount::dest(ReadOnly, d))
        .collect()
}
