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

use std::{env::var, process::id, time::Duration};

use nix::unistd::{getegid, geteuid};
use signal_hook::consts::*;

use crate::{ErrorExt, ErrorKind, format_static, lazy_lock, utils::unix_epoch_time};

pub use crate::utils::ansi::*;

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

lazy_lock! {
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
    pub static ref LOCK_FILE: &'static str = format_static!("{}/pacwrap.lck", *DATA_DIR);
    pub static ref CONTAINER_DIR: &'static str = format_static!("{}/root/", *DATA_DIR);
    pub static ref CACHE_DIR: &'static str = env_default_dir("PACWRAP_CACHE_DIR", PACWRAP_CACHE_DIR);
    pub static ref CONFIG_DIR: &'static str = env_default_dir("PACWRAP_CONFIG_DIR", PACWRAP_CONFIG_DIR);
    pub static ref DATA_DIR: &'static str = env_default_dir("PACWRAP_DATA_DIR", PACWRAP_DATA_DIR);
    pub static ref CONFIG_FILE: &'static str = format_static!("{}/pacwrap.yml", *CONFIG_DIR);
    pub static ref XDG_RUNTIME_DIR: String = format!("/run/user/{}", *UID);
    pub static ref DBUS_SOCKET: String = format!("/run/user/{}/pacwrap_dbus_{}", *UID, &id());
    pub static ref WAYLAND_SOCKET: String = format!("{}/{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);
    pub static ref LOG_LOCATION: &'static str = format_static!("{}/pacwrap.log", *DATA_DIR);
    pub static ref UNIX_TIMESTAMP: u64 = unix_epoch_time().as_secs();
}

fn env(env: &'static str) -> &'static str {
    var(env).map_or_else(|_| ErrorKind::EnvVarUnset(env).fatal(), |var| var.leak())
}

fn env_opt(env: &str) -> &'static str {
    var(env).map_or_else(|_| "", |var| var.leak())
}

fn env_default(env: &str, default: &'static str) -> &'static str {
    var(env).map_or_else(|_| default, |var| var.leak())
}

fn env_default_dir(env: &str, default: &str) -> &'static str {
    var(env).map_or_else(|_| format_static!("{}{}", *HOME, default), |var| var.leak())
}
