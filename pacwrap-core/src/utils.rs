use std::io::Error;
use std::path::Path;
use std::process::Child;
use std::env::var;
use std::os::unix::net::UnixStream;
use std::fmt::Display;

use nix::unistd::isatty;

use crate::ErrorKind;
use crate::constants::{BOLD_RED, BOLD_YELLOW, RESET};

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

pub fn env_var(env: &'static str) -> Result<String, ErrorKind> {
    match var(env) {
        Ok(var) => Ok(var),
        Err(_) => Err(ErrorKind::EnvVarUnset(env))
    }
}

pub fn check_socket(socket: &String) -> bool {
    match UnixStream::connect(&Path::new(socket)) { Ok(_) => true, Err(_) => false, }
}

pub fn handle_process(name: &str, result: Result<Child, Error>) -> Result<(), ErrorKind> {
    match result {
        Ok(child) => Ok(wait_on_process(child)),
        Err(error) => Err(ErrorKind::IOError(name.into(), error.kind())),
    }
}

pub fn is_color_terminal() -> bool {
    let is_dumb = match var("TERM") {
        Ok(value) => value.to_lowercase() != "dumb",
        Err(_) => false,
    };

    is_dumb && isatty(0).is_ok() && isatty(1).is_ok()
}

pub fn is_truecolor_terminal() -> bool {
    let colorterm = match var("COLORTERM") {
        Ok(value) => {
            let value = value.to_lowercase();

            value == "truecolor" || value == "24bit"
        }
        Err(_) => false,
    };

    is_color_terminal() && colorterm
}

pub fn read_le_32(vec: &Vec<u8>, pos: usize) -> u32 {
    ((vec[pos+0] as u32) << 0) + ((vec[pos+1] as u32) << 8) + ((vec[pos+2] as u32) << 16) + ((vec[pos+3] as u32) << 24) 
}

fn wait_on_process(mut child: Child) { 
    child.wait().ok(); 
}
