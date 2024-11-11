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
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        filesystem::{BindError, Filesystem, Mount},
        ContainerVariables,
    },
    err,
    exec::args::ExecutionArgs,
    Error,
    Result,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryFilesystem {
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "volumes")]
    mounts: Vec<Mount>,
}

#[typetag::serde(name = "tmpfs")]
impl Filesystem for TemporaryFilesystem {
    fn qualify(&self, _vars: &ContainerVariables) -> Result<()> {
        if self.mounts.is_empty() {
            err!(BindError::Warn("Mount volumes undeclared.".into()))?
        }

        for m in self.mounts.iter() {
            if m.path.is_empty() {
                err!(BindError::Warn("Mount volumes undeclared.".into()))?
            }
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _: &ContainerVariables) {
        for m in self.mounts.iter() {
            args.tmp(&m.path);
        }
    }

    fn module(&self) -> &'static str {
        "tmpfs"
    }
}
