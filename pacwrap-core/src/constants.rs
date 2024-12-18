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

use std::{env::var, process::id, time::Duration};

use lazy_static::lazy_static;
use nix::unistd::{getegid, geteuid};
use signal_hook::consts::*;

use crate::{
    error,
    utils::{ansi::*, unix_epoch_time},
    Error,
    ErrorKind,
};

pub static PROCESS_SLEEP_DURATION: Duration = Duration::from_millis(250);

pub const BWRAP_EXECUTABLE: &str = "bwrap";
pub const DBUS_PROXY_EXECUTABLE: &str = "xdg-dbus-proxy";
pub const DEFAULT_PATH: &str = "/usr/local/bin:/bin:/usr/bin/";
pub const PACMAN_KEY_SCRIPT: &str = "pacwrap-key";
pub const RUNTIME_DIRECTORY: &str = "/usr/share/pacwrap/runtime";
pub const RUNTIME_TLS_STORE: &str = "/etc/ca-certificates/extracted/tls-ca-bundle.pem";
pub const SIGNAL_LIST: &[i32; 4] = &[SIGHUP, SIGINT, SIGQUIT, SIGTERM];

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
    pub static ref VERBOSE: bool = var("PACWRAP_VERBOSE").is_ok_and(|v| v == "1");
    pub static ref UID: u32 = geteuid().as_raw();
    pub static ref GID: u32 = getegid().as_raw();
    pub static ref HOME: &'static str = env("HOME");
    pub static ref TERM: &'static str = env_opt("TERM");
    pub static ref VERSION_MAJOR: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
    pub static ref VERSION_MINOR: u32 = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
    pub static ref VERSION_PATCH: u32 = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();
    pub static ref COLORTERM: &'static str = env_opt("COLORTERM");
    pub static ref LANG: &'static str = env_default("LANG", "en_US.UTF-8");
    pub static ref WAYLAND_DISPLAY: &'static str = env_opt("WAYLAND_DISPLAY");
    pub static ref EDITOR: &'static str = env_default("EDITOR", "vi");
    pub static ref X11_DISPLAY: &'static str = env_opt("DISPLAY");
    pub static ref XAUTHORITY: &'static str = env_opt("XAUTHORITY");
    pub static ref LOCK_FILE: &'static str = format_str!("{}/pacwrap.lck", *DATA_DIR);
    pub static ref CONTAINER_DIR: &'static str = format_str!("{}/root/", *DATA_DIR);
    pub static ref CACHE_DIR: &'static str = env_default_dir("PACWRAP_CACHE_DIR", PACWRAP_CACHE_DIR);
    pub static ref CONFIG_DIR: &'static str = env_default_dir("PACWRAP_CONFIG_DIR", PACWRAP_CONFIG_DIR);
    pub static ref DATA_DIR: &'static str = env_default_dir("PACWRAP_DATA_DIR", PACWRAP_DATA_DIR);
    pub static ref CONFIG_FILE: &'static str = format_str!("{}/pacwrap.yml", *CONFIG_DIR);
    pub static ref XDG_RUNTIME_DIR: String = format!("/run/user/{}", *UID);
    pub static ref DBUS_SOCKET: String = format!("/run/user/{}/pacwrap_dbus_{}", *UID, &id());
    pub static ref WAYLAND_SOCKET: String = format!("{}/{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);
    pub static ref LOG_LOCATION: &'static str = format_str!("{}/pacwrap.log", *DATA_DIR);
    pub static ref UNIX_TIMESTAMP: u64 = unix_epoch_time().as_secs();
    pub static ref IS_COLOR_TERMINAL: bool = is_color_terminal();
    pub static ref IS_TRUECOLOR_TERMINLAL: bool = is_truecolor_terminal();
    pub static ref BOLD: &'static str = bold();
    pub static ref RESET: &'static str = reset();
    pub static ref DIM: &'static str = dim();
    pub static ref YELLOW: &'static str = yellow();
    pub static ref CHECKMARK: &'static str = checkmark();
    pub static ref BOLD_WHITE: &'static str = bold_white();
    pub static ref BOLD_YELLOW: &'static str = bold_yellow();
    pub static ref BOLD_RED: &'static str = bold_red();
    pub static ref BOLD_GREEN: &'static str = bold_green();
    pub static ref BAR_GREEN: &'static str = bar_green();
    pub static ref BAR_CYAN: &'static str = bar_cyan();
    pub static ref BAR_RED: &'static str = bar_red();
    pub static ref ARROW_CYAN: &'static str = arrow_cyan();
    pub static ref ARROW_RED: &'static str = arrow_red();
    pub static ref ARROW_GREEN: &'static str = arrow_green();
    pub static ref UNDERLINE: &'static str = underline();
}

fn env(env: &'static str) -> &'static str {
    var(env).map_or_else(|_| error!(ErrorKind::EnvVarUnset(env)).fatal(), |var| var.leak())
}

fn env_opt(env: &str) -> &'static str {
    var(env).map_or_else(|_| "", |var| var.leak())
}

fn env_default(env: &str, default: &'static str) -> &'static str {
    var(env).map_or_else(|_| default, |var| var.leak())
}

fn env_default_dir(env: &str, default: &str) -> &'static str {
    var(env).map_or_else(|_| format_str!("{}{}", *HOME, default), |var| var.leak())
}
