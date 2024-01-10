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

use std::{path::Path, env::var, os::unix::net::UnixStream, fmt::Display, time::{SystemTime, UNIX_EPOCH}};

use nix::unistd::isatty;

use crate::{err, Error, ErrorKind, Result, constants::{BOLD_RED, BOLD_YELLOW, RESET, TERM, COLORTERM, UID, GID}};

pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod termcontrol;
pub mod arguments;
pub mod prompt;

pub fn print_warning(message: impl Into<String> + Display) {
    eprintln!("{}warning:{} {}", *BOLD_YELLOW, *RESET,  &message);
} 

pub fn print_error(message: impl Into<String> + Display) {
    eprintln!("{}error:{} {}", *BOLD_RED, *RESET, &message);
} 

pub fn env_var(env: &'static str) -> Result<String> {
    match var(env) {
        Ok(var) => Ok(var),
        Err(_) => err!(ErrorKind::EnvVarUnset(env))
    }
}

pub fn check_socket(socket: &String) -> bool {
    match UnixStream::connect(&Path::new(socket)) { Ok(_) => true, Err(_) => false, }
}

pub fn is_color_terminal() -> bool {
    let value = *TERM;
    let is_dumb = ! value.is_empty() && value.to_lowercase() != "dumb";

    is_dumb && isatty(0).is_ok() && isatty(1).is_ok()
}

pub fn is_truecolor_terminal() -> bool {
    let value = COLORTERM.to_lowercase();

    is_color_terminal() && value == "truecolor" || value == "24bit"

}

pub fn unix_time_as_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn read_le_32(vec: &Vec<u8>, pos: usize) -> u32 {
    ((vec[pos+0] as u32) << 0) + ((vec[pos+1] as u32) << 8) + ((vec[pos+2] as u32) << 16) + ((vec[pos+3] as u32) << 24) 
}

pub fn check_root() -> Result<()> {
    if *UID == 0 || *GID == 0 {
        err!(ErrorKind::ElevatedPrivileges)?
    }

    Ok(())
}
