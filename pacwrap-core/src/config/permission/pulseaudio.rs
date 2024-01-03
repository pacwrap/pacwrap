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
    config::{Permission, permission::*},
    config::permission::{Condition::Success, PermError::Warn},
    constants::XDG_RUNTIME_DIR,
    utils::check_socket};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PULSEAUDIO {
    #[serde(skip_serializing_if = "is_default_socket", default = "default_socket")]
    socket: String,
}

#[typetag::serde]
impl Permission for PULSEAUDIO {
    fn check(&self) -> Result<Option<Condition>, PermError> {  
        if ! Path::new(&self.socket).exists() {
            Err(Warn(format!("Pulseaudio socket not found.")))?
        }

        if ! check_socket(&self.socket) {          
            Err(Warn(format!("'{}' is not a valid UNIX socket.", &self.socket)))?
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs) {
        args.robind(&self.socket, default_socket());
    }

    fn module(&self) -> &'static str {
        "PULSEAUDIO"
    }
}

fn is_default_socket(var: &String) -> bool {
    let default: &String = &default_socket();
    default == var
}

fn default_socket() -> String {
    format!("{}/pulse/native", *XDG_RUNTIME_DIR)
}
