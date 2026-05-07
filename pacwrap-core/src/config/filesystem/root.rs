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

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    Result,
    config::{
        ContainerVariables,
        filesystem::{BindError, Filesystem, Mount, Permission::ReadOnly},
    },
    eprintln_warn,
    exec::args::ExecutionArgs,
};

const VOLUMES: [&str; 2] = ["/usr", "/etc"];
const SYMLINKS: [(&str, &str); 4] = [
    ("/usr/lib", "/lib"),
    ("/usr/lib", "/lib64"),
    ("/usr/bin", "/bin"),
    ("/usr/bin", "/sbin"),
];

impl Default for Root {
    fn default() -> Root {
        Root { mounts: volumes() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    #[serde(skip_serializing_if = "is_volumes", default = "volumes", rename = "volumes")]
    mounts: Vec<Mount>,
}

#[typetag::serde(name = "root")]
impl Filesystem for Root {
    fn qualify(&self, vars: &ContainerVariables) -> Result<()> {
        let volumes = volumes();

        if !Path::new(vars.root()).exists() {
            Err(BindError::Fail(format!("Container {} not found. ", vars.instance())))?
        }

        if self.mounts.is_empty() {
            Err(BindError::Fail("Mount volumes are undeclared.".to_string()))?
        }

        for mount in volumes.iter() {
            if mount.path.is_empty() {
                Err(BindError::Warn("Path is undeclared.".into()))?
            }

            if let Err(err) = check_mount(vars, mount) {
                eprintln_warn!("{err}");
            }
        }

        for mount in self.mounts.iter().filter(|a| !volumes.contains(a)) {
            if mount.path.is_empty() {
                Err(BindError::Warn("Path is undeclared.".into()))?
            }

            check_mount(vars, mount)?;
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &ContainerVariables) {
        let mounts = &volumes();

        for mount in mounts {
            args.bind(&mount.permission, &format!("{}{}", vars.root(), mount.path), &mount.path);
        }

        for mount in self.mounts.iter().filter(|a| !mounts.contains(a) && check_mount(vars, a).is_ok()) {
            args.bind(&mount.permission, &format!("{}{}", vars.root(), mount.path), &mount.path);
        }

        for (dest, src) in SYMLINKS {
            args.symlink(dest, src);
        }
    }

    fn module(&self) -> &'static str {
        "root"
    }
}

fn check_mount(vars: &ContainerVariables, mount: &Mount) -> Result<()> {
    if !Path::new(&format!("{}/{}", vars.root(), &mount.path)).exists() {
        Err(BindError::Fail(format!("{} not found in container root.", &mount.path)))?
    }

    Ok(())
}

fn is_volumes(vols: &[Mount]) -> bool {
    vols == volumes()
}

fn volumes() -> Vec<Mount> {
    VOLUMES.iter().map(|d| Mount::dest(ReadOnly, d)).collect()
}
