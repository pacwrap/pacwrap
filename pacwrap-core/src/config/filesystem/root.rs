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
pub struct ROOT;

#[typetag::serde]
impl Filesystem for ROOT {
    fn check(&self, vars: &InsVars) -> Result<(), BindError> {
        if ! Path::new(vars.root()).exists() {
            Err(BindError::Fail(format!("Container {} not found. ", vars.instance())))?
        }
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) { 
        args.robind(format!("{}/usr", vars.root()), "/usr");
        args.robind(format!("{}/etc", vars.root()), "/etc");
        args.symlink("/usr/lib", "/lib");
        args.symlink("/usr/lib", "/lib64");
        args.symlink("/usr/bin", "/bin");
        args.symlink("/usr/bin", "/sbin");
    }

    fn module(&self) -> &'static str {
        "ROOT"
    }
}
