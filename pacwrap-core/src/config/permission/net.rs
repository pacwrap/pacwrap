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
        permission::{Condition::Success, *},
        Permission,
    },
    exec::args::{Argument::HostNetworking, ExecutionArgs},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network;

#[typetag::serde(name = "net")]
impl Permission for Network {
    fn qualify(&self) -> Result<Option<Condition>, PermError> {
        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        args.push_env(HostNetworking);
        args.bind("/etc/resolv.conf", "/etc/resolv.conf");
    }

    fn module(&self) -> &'static str {
        "net"
    }
}
