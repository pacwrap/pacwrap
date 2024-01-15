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

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        filesystem::{BindError, Filesystem},
        InsVars,
    },
    exec::args::ExecutionArgs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dir {
    #[serde(default)]
    path: Vec<String>,
}

#[typetag::serde(name = "dir")]
impl Filesystem for Dir {
    fn check(&self, _vars: &InsVars) -> Result<(), BindError> {
        if self.path.len() == 0 {
            Err(BindError::Fail(format!("Path not specified.")))?
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _vars: &InsVars) {
        for dir in self.path.iter() {
            args.dir(dir);
        }
    }

    fn module(&self) -> &'static str {
        "dir"
    }
}
