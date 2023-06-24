use std::path::Path;
use std::process::{Child, exit};
use std::env::var;
use std::os::unix::net::UnixStream;
use std::fmt::Display;

use lazy_static::lazy_static;
use serde_json::Value;
use nix::unistd::isatty;

use crate::config::vars::InsVars;

pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod termcontrol;
pub mod arguments;

lazy_static! {
    static ref ISATTY: bool = isatty_ansi(2);
}

pub fn test_root(instance: &InsVars) {
    if ! Path::new(&instance.root()).exists() || ! Path::new(&instance.home()).exists() {  
        print_error(format!("Target container {}: not found.", instance.instance()));
        exit(2);
    }
}

fn isatty_ansi(fd: i32) -> bool {
    match isatty(fd) {
        Ok(b) => {
            if b && var("TERM").unwrap() != "dumb" {
                true
            } else {
                false
            }
        },
        Err(_) => false
    }
}

pub fn print_warning(message: impl Into<String> + Display) {
    if *ISATTY {
        eprintln!("[1m[93mwarning:[0m {}", &message);
    } else {
        eprintln!("WARNING: {}", &message);
    }
} 

pub fn print_error(message: impl Into<String> + Display) {
    if *ISATTY { 
        eprintln!("[1m[31merror:[0m {}", &message);
    } else {
        eprintln!("ERROR: {}", &message);
    }
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

pub fn job_i32(process: &mut Child) -> i32 {
    match process.id().try_into() { Ok(i) => i, Err(_) => 0 }
}

pub fn derive_bwrap_child(value: &Value) -> i32 {
    match serde_json::from_value(value.clone()) { Ok(u) => u, Err(_) => 0 }
}

pub fn print_help_msg(args: &str) {
    println!("pacwrap error: {} ", args);
    println!("Try 'pacwrap -h' for more information on valid operational parameters.");
    exit(1);
}
