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
        permission::{Condition::Success, PermError::Fail, *},
        Permission,
    },
    exec::args::ExecutionArgs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Dev {
    devices: Vec<String>,
}

#[typetag::serde(name = "dev")]
impl Permission for Dev {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        for device in self.devices.iter() {
            if !Path::new(&format!("/dev/{}", device)).exists() {
                Err(Fail(format!("/dev/{} is inaccessible.", device)))?
            }
        }

        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        for device in self.devices.iter() {
            args.dev(&format!("/dev/{}", device));
        }
    }

    fn module(&self) -> &'static str {
        "DEV"
    }
}
