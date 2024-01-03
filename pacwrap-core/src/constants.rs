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

use std::{env::var, process::id};

use lazy_static::lazy_static;
use nix::unistd::{geteuid, getegid};

use crate::{error, Error, ErrorKind, utils::{is_color_terminal, is_truecolor_terminal}};

pub const BWRAP_EXECUTABLE: &str = "bwrap";
pub const DBUS_PROXY_EXECUTABLE: &str = "xdg-dbus-proxy";
pub const DEFAULT_PATH: &str = "/usr/local/bin:/usr/bin/:/bin";

const PACWRAP_CONFIG_DIR: &str = "/.config/pacwrap";
const PACWRAP_DATA_DIR: &str = "/.local/share/pacwrap";
const PACWRAP_CACHE_DIR: &str = "/.cache/pacwrap";

#[macro_export]
macro_rules! format_str {
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

lazy_static! {
    pub static ref UID: u32 = geteuid().as_raw();
    pub static ref GID: u32 = getegid().as_raw();
    pub static ref PWD: &'static str = env_opt("PWD"); 
    pub static ref HOME: &'static str = env("HOME");
    pub static ref USER: &'static str = env("USER");
    pub static ref TERM: &'static str = env_opt("TERM");
    pub static ref COLORTERM: &'static str = env_opt("COLORTERM");
    pub static ref LANG: &'static str = env_default("LAMG", "en_US.UTF-8");
    pub static ref WAYLAND_DISPLAY: &'static str = env_opt("WAYLAND_DISPLAY");
    pub static ref X11_DISPLAY: &'static str = env_opt("DISPLAY"); 
    pub static ref XAUTHORITY: &'static str = env_opt("XAUTHORITY");
    pub static ref CACHE_DIR: &'static str = env_default_dir("PACWRAP_CACHE_DIR", PACWRAP_CACHE_DIR);
    pub static ref CONFIG_DIR: &'static str = env_default_dir("PACWRAP_CONFIG_DIR", PACWRAP_CONFIG_DIR);
    pub static ref DATA_DIR: &'static str = env_default_dir("PACWRAP_DATA_DIR", PACWRAP_DATA_DIR);
    pub static ref GLOBAL_CONFIG: &'static str = format_str!("{}/pacwrap.yml", PACWRAP_CONFIG_DIR); 
    pub static ref PACWRAP_AGENT_FILE: &'static str = format_str!("/run/user/{}/pacwrap_agent_{}", *UID, &id()); 
    pub static ref XDG_RUNTIME_DIR: String = format!("/run/user/{}", *UID);
    pub static ref DBUS_SOCKET: String = format!("/run/user/{}/pacwrap_dbus_{}", *UID, &id());
    pub static ref WAYLAND_SOCKET: String = format!("{}{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);
    pub static ref LOG_LOCATION: &'static str = format_str!("{}/pacwrap.log", *DATA_DIR);
    pub static ref IS_COLOR_TERMINLAL: bool = is_color_terminal();
    pub static ref IS_TRUECOLOR_TERMINLAL: bool = is_truecolor_terminal();
    pub static ref BOLD: &'static str = bold();
    pub static ref RESET: &'static str = reset();
    pub static ref DIM: &'static str = dim();
    pub static ref BOLD_WHITE: &'static str = bold_white();
    pub static ref BOLD_YELLOW: &'static str = bold_yellow();
    pub static ref BOLD_RED: &'static str = bold_red();
    pub static ref BOLD_GREEN: &'static str = bold_green(); 
    pub static ref BAR_GREEN: &'static str = bar_green(); 
    pub static ref BAR_CYAN: &'static str = bar_cyan(); 
    pub static ref ARROW_CYAN: &'static str = arrow_cyan();  
    pub static ref ARROW_RED: &'static str = arrow_red(); 
    pub static ref ARROW_GREEN: &'static str = arrow_green(); 
}

fn arrow_red() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;31m->[0m" } else { "->" }
}

fn arrow_cyan() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;36m->[0m" } else { "->" }
}

fn arrow_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m->[0m" } else { "->" }
}

fn bar_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m::[0m" } else { "::" }
}

fn bar_cyan() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;36m::[0m" } else { "::" }
}

fn dim() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[2m" } else { "" }
}

fn bold() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1m" } else { "" }
}

fn bold_white() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;37m" } else { "" }
}

fn bold_red() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;31m" } else { "" }
}

fn bold_yellow() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;33m" } else { "" }
}

fn bold_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m" } else { "" }
}

fn reset() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[0m" } else { "" }
}

fn env(env: &'static str) -> &'static str {
    match var(env) {
        Ok(var) => var.leak(), Err(_) => { error!(ErrorKind::EnvVarUnset(env)).handle(); "" }
    }
}

fn env_opt(env: &str) -> &'static str {
    match var(env) {
        Ok(var) => var.leak(), Err(_) => "",
    }
}

fn env_default(env: &str, default: &'static str) -> &'static str {
    match var(env) {
        Ok(var) => var.leak(), Err(_) => default, 
    }
}

fn env_default_dir(env: &str, default: &str) -> &'static str {
    match var(env) {
        Ok(var) => var.leak(), Err(_) => format_str!("{}{}", *HOME, default), 
    }
}
