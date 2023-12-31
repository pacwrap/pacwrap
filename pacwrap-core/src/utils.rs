use std::path::Path;
use std::env::var;
use std::os::unix::net::UnixStream;
use std::fmt::Display;

use nix::unistd::isatty;

use crate::{Error, ErrorKind, err};
use crate::constants::{BOLD_RED, BOLD_YELLOW, RESET, TERM, COLORTERM};

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

pub fn env_var(env: &'static str) -> Result<String, Error> {
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

pub fn read_le_32(vec: &Vec<u8>, pos: usize) -> u32 {
    ((vec[pos+0] as u32) << 0) + ((vec[pos+1] as u32) << 8) + ((vec[pos+2] as u32) << 16) + ((vec[pos+3] as u32) << 24) 
}
