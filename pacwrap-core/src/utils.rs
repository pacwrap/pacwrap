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

use std::{
    env::var,
    os::unix::net::UnixStream,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    constants::{BOLD_RED, BOLD_YELLOW, GID, RESET, UID},
    err,
    Error,
    ErrorKind,
    Result,
};

pub use ansi::{is_color_terminal, is_truecolor_terminal};
pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod ansi;
pub mod arguments;
pub mod bytebuffer;
pub mod prompt;
pub mod table;
pub mod termcontrol;

pub fn print_warning(message: &str) {
    eprintln!("{}warning:{} {}", *BOLD_YELLOW, *RESET, message);
}

pub fn print_error(message: &str) {
    eprintln!("{}error:{} {}", *BOLD_RED, *RESET, message);
}

pub fn check_socket(socket: &String) -> bool {
    UnixStream::connect(&Path::new(socket)).is_ok()
}

pub fn unix_time_as_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub fn whitespace(amt: usize) -> String {
    " ".repeat(amt)
}

pub fn env_var(env: &'static str) -> Result<String> {
    match var(env) {
        Ok(var) => Ok(var),
        Err(_) => err!(ErrorKind::EnvVarUnset(env)),
    }
}

pub fn check_root() -> Result<()> {
    if *UID == 0 || *GID == 0 {
        err!(ErrorKind::ElevatedPrivileges)?
    }

    Ok(())
}
