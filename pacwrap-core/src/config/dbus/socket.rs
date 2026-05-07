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
use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Socket {
    policy: String,
    address: Vec<String>,
}

#[typetag::serde(name = "socket")]
impl Dbus for Socket {
    fn register(&self, args: &mut ExecutionArgs) {
        match self.policy.to_lowercase().as_str() {
            p if p == "call" || p == "talk" || p == "see" || p == "own" || p == "broadcast" =>
                for sock in self.address.iter() {
                    args.dbus(p, sock);
                },
            _ => {}
        }
    }
}
