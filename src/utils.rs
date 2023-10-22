use std::io::Error;
use std::path::Path;
use std::process::{Child, exit};
use std::env::var;
use std::os::unix::net::UnixStream;
use std::fmt::Display;

use nix::unistd::isatty;

use crate::constants::{BOLD_RED, BOLD_YELLOW, RESET};
use crate::config::vars::InsVars;

pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod termcontrol;
pub mod arguments;
pub mod prompt;

pub fn test_root(instance: &InsVars) {
    if ! Path::new(instance.root().as_ref()).exists() {  
        print_error(format!("Target container {}: not found.", instance.instance()));
        exit(2);
    }
}

pub fn print_warning(message: impl Into<String> + Display) {
    eprintln!("{}warning:{} {}", *BOLD_YELLOW, *RESET,  &message);
} 

pub fn print_error(message: impl Into<String> + Display) {
    eprintln!("{}error:{} {}", *BOLD_RED, *RESET, &message);
} 

pub fn env_var(env: &str) -> String {
    match var(env) {
        Ok(var) => var,
        Err(_) => { print_error(format!("${} is not set.", env)); exit(2); }
    }
}

pub fn check_socket(socket: &String) -> bool {
    match UnixStream::connect(&Path::new(socket)) { Ok(_) => true, Err(_) => false, }
}

pub fn print_help_error(args: impl Into<String>) {
    print_error(args.into());
    println!("Try 'pacwrap -h' for more information on valid operational parameters.");
    exit(1);
}

#[allow(dead_code)]
pub fn print_help_msg(args: impl Into<String>) {
    println!("pacwrap error: {} ", args.into());
    println!("Try 'pacwrap -h' for more information on valid operational parameters.");
    exit(1);
}

pub fn handle_process(result: Result<Child, Error>) {
    match result {
        Ok(child) => wait_on_process(child),
        Err(_) => print_error("Failed to spawn child process."),
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

fn wait_on_process(mut child: Child) { 
    child.wait().ok(); 
}
