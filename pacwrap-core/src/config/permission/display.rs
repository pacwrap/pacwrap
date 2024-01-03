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
    config::permission::{
        Condition::{Success, SuccessWarn}, 
        PermError::Fail},
    constants::{XDG_RUNTIME_DIR, WAYLAND_SOCKET, WAYLAND_DISPLAY, X11_DISPLAY, XAUTHORITY}, 
    utils::check_socket};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DISPLAY;

#[typetag::serde]
impl Permission for DISPLAY {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        let mut bound = None;

        match validate_wayland_socket() { 
            Ok(b) => if let Some(con) = b { 
                bound = Some(con);
            },
            Err(e) => Err(e)?
        }

        match validate_xorg_socket() { 
            Ok(b) => if let Some(con) = b { 
                bound = Some(con);
            },
            Err(e) => Err(e)?
        }

        if let None = bound {
            Err(Fail(format!("Expected environment variables were not found.")))
        } else {
            Ok(bound)
        }
   }
    
    fn register(&self, args: &mut  ExecutionArgs) {
        if ! WAYLAND_DISPLAY.is_empty() { 
            configure_wayland(args); 
        } 

        if ! X11_DISPLAY.is_empty() { 
            configure_xorg(args); 
        }          
    }

    fn module(&self) -> &'static str {
        "DISPLAY"
    }
}

fn validate_wayland_socket() -> Result<Option<Condition>, PermError> {
    if ! WAYLAND_DISPLAY.is_empty() {
        if ! Path::new(&*WAYLAND_SOCKET).exists() { 
            Err(Fail(format!("Wayland socket '{}' not found.", &*WAYLAND_SOCKET)))?
        }

        if ! check_socket(&*WAYLAND_SOCKET) { 
            Err(Fail(format!("'{}' is not a valid UNIX socket.", &*WAYLAND_SOCKET)))?
        }

        return Ok(Some(Success));
    }

    Ok(None)
}

fn validate_xorg_socket() -> Result<Option<Condition>, PermError> {  
    if ! X11_DISPLAY.is_empty() { 
        let display: Vec<&str> = X11_DISPLAY.split(":").collect();
        let xorg_socket = format!("/tmp/.X11-unix/X{}", display[1]);
             
        if XAUTHORITY.is_empty() {
            Err(Fail(format!("XAUTHORITY environment variable unspecified.")))? 
        }
        
        if ! Path::new(*XAUTHORITY).exists() { 
            Err(Fail(format!("Xauthority file '{}' not found.", *XAUTHORITY)))?
        }
         
        if display[0].is_empty() || display[0] == "unix" {  
            if Path::new(&xorg_socket).exists() {
                if ! check_socket(&xorg_socket) {  
                    Err(Fail(format!("'{}' is not a valid UNIX socket.", &xorg_socket)))?
                }
        
                return Ok(Some(Success));
            } else {
                Err(Fail(format!("X11 socket '{}' not found.", &xorg_socket)))?
            } 
        } else { 
            return Ok(Some(SuccessWarn(format!("Connecting to TCP X11 socket at '{}'", *X11_DISPLAY))));
        }
    }

    Ok(None)
}

fn configure_wayland(args: &mut ExecutionArgs) {
    let wayland_socket = format!("{}/{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);  

    args.env("WAYLAND_DISPLAY", *WAYLAND_DISPLAY);
    args.robind(&wayland_socket, &wayland_socket);
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
