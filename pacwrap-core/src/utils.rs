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

use std::{
    env::var,
    os::unix::net::UnixStream,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    ErrorKind,
    Result,
    constants::{GID, UID},
};

pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod ansi;
pub mod arguments;
pub mod bytebuffer;
pub mod prompt;
pub mod table;
pub mod termcontrol;

#[macro_export]
macro_rules! lazy_lock {
    ( $v:vis static ref $x:ident: $y:ty = $z: expr; $($t:tt)* ) => {
        $v static $x: std::sync::LazyLock<$y> = std::sync::LazyLock::new(|| $z);
        lazy_lock!($($t)*);
    };
    () => ()
}

#[macro_export]
macro_rules! eprintln_warn {
    ( $( $x:expr ),+  ) => {
        {
            eprint!("{}warning:{} ", *$crate::utils::ansi::BOLD_YELLOW, *$crate::utils::ansi::RESET);
            eprintln!($( $x, )+);
        }
    };
}

#[macro_export]
macro_rules! eprintln_error {
    ( $( $x:expr ),+ ) => {
        {
            eprint!("{}error:{} ", *$crate::utils::ansi::BOLD_RED, *$crate::utils::ansi::RESET);
            eprintln!($( $x, )+);
        }
    };
}

#[macro_export]
macro_rules! eprintln_fatal {
    ( $( $x:expr ),+ ) => {
        {
            eprint!("{}fatal:{} ", *$crate::utils::ansi::BOLD_RED, *$crate::utils::ansi::RESET);
            eprintln!($( $x, )+);
        }
    };
}

#[macro_export]
macro_rules! format_static {
    ( $( $x:expr ),+ ) => {
        format!($( $x, )+).leak()
    };
}

#[macro_export]
macro_rules! to_static_str {
    ( $x:expr ) => {
        $x.to_string().leak()
    };
}

pub fn env_var(env: &'static str) -> Result<String> {
    match var(env) {
        Ok(var) => Ok(var),
        Err(_) => Err(ErrorKind::EnvVarUnset(env))?,
    }
}

pub fn check_root() -> Result<()> {
    if *UID == 0 || *GID == 0 {
        Err(ErrorKind::ElevatedPrivileges)?
    }

    Ok(())
}

pub fn check_socket(socket: &String) -> bool {
    UnixStream::connect(Path::new(socket)).is_ok()
}

pub fn unix_epoch_time() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("SystemTime")
}
