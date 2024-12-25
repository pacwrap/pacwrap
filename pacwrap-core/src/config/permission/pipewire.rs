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
use serde_with::skip_serializing_none;

use crate::{
    config::{
        filesystem::Permission::ReadOnly,
        permission::{Condition::Success, PermError::Warn, *},
        Permission,
    },
    constants::XDG_RUNTIME_DIR,
    exec::args::ExecutionArgs,
    utils::check_socket,
};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pipewire {
    socket: Option<String>,
}

#[typetag::serde(name = "pipewire")]
impl Permission for Pipewire {
    fn qualify(&self) -> Result<Option<Condition>, PermError> {
        let default = &default_socket();
        let socket = self.socket.as_ref().unwrap_or(default);

        if !Path::new(socket).exists() {
            Err(Warn("Pipewire socket not found.".to_string()))?
        }

        if !check_socket(socket) {
            Err(Warn(format!("'{}' is not a valid UNIX socket.", &socket)))?
        }

        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        let default = &default_socket();
        let socket = self.socket.as_ref().unwrap_or(default);

        args.bind(&ReadOnly, socket, default);
    }

    fn module(&self) -> &'static str {
        "pipewire"
    }
}

fn default_socket() -> String {
    format!("{}/pipewire-0", *XDG_RUNTIME_DIR)
}
