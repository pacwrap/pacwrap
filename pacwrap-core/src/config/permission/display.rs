use std::path::Path;
use std::env::var;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::constants::XDG_RUNTIME_DIR;
use crate::utils::check_socket;
use crate::exec::args::ExecutionArgs;
use crate::config::{Permission, permission::*};
use crate::config::permission::{Condition::{Success, SuccessWarn}, PermError::Fail};

lazy_static! {
    static ref WAYLAND_DISPLAY: String = env_var("WAYLAND_DISPLAY");
    static ref XAUTHORITY: String = env_var("XAUTHORITY");
    static ref DISPLAY_ENV: String = env_var("DISPLAY");
    static ref WAYLAND_SOCKET: String = format!("{}{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DISPLAY;

#[typetag::serde]
impl Permission for DISPLAY {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        let mut bound = None;

        match validate_wayland_socket() { 
            Ok(b) => { 
                if let Some(con) = b { bound = Some(con); }
            },
            Err(e) => Err(e)?
        }

        match validate_xorg_socket() { 
            Ok(b) => {
                if let Some(con) = b { bound = Some(con); }
            },
            Err(e) => Err(e)?
        }

        if let None = bound {
            Err(Fail(format!("Expected environment variables are unspecified.")))
        } else {
            Ok(bound)
        }
   }
    
    fn register(&self, args: &mut  ExecutionArgs) {
        if ! WAYLAND_DISPLAY.is_empty() { 
            configure_wayland(args); 
        } 

        if ! DISPLAY_ENV.is_empty() { 
            configure_xorg(args); 
        }          
    }

    fn module(&self) -> &str {
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
    if ! DISPLAY_ENV.is_empty() { 
        let display: Vec<&str> = DISPLAY_ENV.split(":").collect();
        let xorg_socket = format!("/tmp/.X11-unix/X{}", display[1]);
             
        if XAUTHORITY.is_empty() {
            Err(Fail(format!("XAUTHORITY environment variable unspecified.")))? 
        }
        
        if ! Path::new(&*XAUTHORITY).exists() { 
            Err(Fail(format!("Xauthority file '{}' not found.",&*XAUTHORITY)))?
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
            return Ok(Some(SuccessWarn(format!("Connecting to TCP X11 socket at '{}'", *DISPLAY_ENV))));
        }
    }
    Ok(None)
}

fn configure_wayland(args: &mut ExecutionArgs) {
    let wayland_socket = format!("{}/{}", *XDG_RUNTIME_DIR, *WAYLAND_DISPLAY);  

    args.env("WAYLAND_DISPLAY", &*WAYLAND_DISPLAY);
    args.robind(&wayland_socket, &wayland_socket);
}

fn configure_xorg(args: &mut ExecutionArgs) {
    let display: Vec<&str> = DISPLAY_ENV.split(":").collect();
    let xorg_socket = format!("/tmp/.X11-unix/X{}", display[1]);
    let container_xauth = format!("{}/Xauthority", *XDG_RUNTIME_DIR);  
        
    args.env("DISPLAY", &*DISPLAY_ENV);
    args.env("XAUTHORITY", &container_xauth); 
    args.robind(&*XAUTHORITY, &container_xauth);
    if display[0].is_empty() || display[0] == "unix" {    
        args.robind(&xorg_socket, &xorg_socket);
    }
}

fn env_var(arg: &str) -> String {
    match var(arg) { Ok(env) => env, Err(_) => String::new() }
}
