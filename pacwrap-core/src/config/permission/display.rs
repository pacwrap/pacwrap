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

use crate::{
    config::{
        permission::{
            Condition::{Success, SuccessWarn},
            PermError::Fail,
            *,
        },
        Permission,
    },
    constants::{WAYLAND_DISPLAY, WAYLAND_SOCKET, X11_DISPLAY, XAUTHORITY, XDG_RUNTIME_DIR},
    exec::args::ExecutionArgs,
    utils::check_socket,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Display;

#[typetag::serde(name = "display")]
impl Permission for Display {
    fn qualify(&self) -> Result<Option<Condition>, PermError> {
        let (wayland, xorg) = (validate_wayland_socket()?, validate_xorg_socket()?);

        if wayland.is_some() {
            Ok(wayland)
        } else if xorg.is_some() {
            Ok(xorg)
        } else {
            Err(Fail("Expected environment variables were not found.".into()))
        }
    }

    fn register(&self, args: &mut ExecutionArgs) {
        if !WAYLAND_DISPLAY.is_empty() {
            configure_wayland(args);
        }

        if !X11_DISPLAY.is_empty() {
            configure_xorg(args);
        }
    }

    fn module(&self) -> &'static str {
        "display"
    }
}

fn validate_wayland_socket() -> Result<Option<Condition>, PermError> {
    if WAYLAND_DISPLAY.is_empty() {
        return Ok(None);
    }

    if !Path::new(&*WAYLAND_SOCKET).exists() {
        Err(Fail(format!("Wayland socket '{}' not found.", &*WAYLAND_SOCKET)))?
    }

    if !check_socket(&WAYLAND_SOCKET) {
        Err(Fail(format!("'{}' is not a valid UNIX socket.", &*WAYLAND_SOCKET)))?
    }

    Ok(Some(Success))
}

fn validate_xorg_socket() -> Result<Option<Condition>, PermError> {
    if X11_DISPLAY.is_empty() {
        return Ok(None);
    }

    if !X11_DISPLAY.contains(':') {
        Err(Fail(format!("Expected value with colon delimiter: `DISPLAY={}`.", *X11_DISPLAY)))?
    }

    if XAUTHORITY.is_empty() {
        Err(Fail("XAUTHORITY environment variable unspecified.".into()))?
    }

    if !Path::new(*XAUTHORITY).exists() {
        Err(Fail(format!("Xauthority file '{}' not found.", *XAUTHORITY)))?
    }

    let display: Vec<&str> = X11_DISPLAY.split(":").collect();
    let xorg_socket = format!("/tmp/.X11-unix/X{}", display[1]);

    if display[0].is_empty() || display[0] == "unix" {
        if Path::new(&xorg_socket).exists() {
            if !check_socket(&xorg_socket) {
                Err(Fail(format!("'{}' is not a valid UNIX socket.", &xorg_socket)))?
            }

            Ok(Some(Success))
        } else {
            Err(Fail(format!("X11 socket '{}' not found.", &xorg_socket)))?
        }
    } else {
        Ok(Some(SuccessWarn(format!("Connecting to TCP X11 socket at '{}'", *X11_DISPLAY))))
    }
}

fn configure_wayland(args: &mut ExecutionArgs) {
    let wayland_socket = format!("{}/wayland-0", *XDG_RUNTIME_DIR);

    args.env("WAYLAND_DISPLAY", "wayland-0");
    args.robind(&WAYLAND_SOCKET, &wayland_socket);
}

fn configure_xorg(args: &mut ExecutionArgs) {
    let display: Vec<&str> = X11_DISPLAY.split(":").collect();
    let xorg_socket = format!("/tmp/.X11-unix/X{}", display[1]);
    let container_xauth = format!("{}/Xauthority", *XDG_RUNTIME_DIR);

    args.env("DISPLAY", *X11_DISPLAY);
    args.env("XAUTHORITY", &container_xauth);
    args.robind(*XAUTHORITY, &container_xauth);

    if display[0].is_empty() || display[0] == "unix" {
        args.robind(&xorg_socket, &xorg_socket);
    }
}
