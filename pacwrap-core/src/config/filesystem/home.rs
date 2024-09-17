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
        filesystem::{BindError, Filesystem},
        ContainerVariables,
    },
    exec::args::ExecutionArgs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Home;

#[typetag::serde(name = "home")]
impl Filesystem for Home {
    fn check(&self, vars: &ContainerVariables) -> Result<(), BindError> {
        if !Path::new(vars.home()).exists() {
            Err(BindError::Fail("Specified home directory not found.".into()))?
        }
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &ContainerVariables) {
        args.bind(vars.home(), vars.home_mount());
        args.env("HOME", vars.home_mount());
        args.env("USER", vars.user());
    }

    fn module(&self) -> &'static str {
        "home"
    }
}
