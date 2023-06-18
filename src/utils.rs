use std::path::Path;
use std::process::exit;
use nix::unistd::isatty;


use crate::config::vars::InsVars;

pub mod arguments;

pub fn test_root(instance: &InsVars) {
    if ! Path::new(&instance.root()).exists() || ! Path::new(&instance.home()).exists() {  
        print_error(format!("Target container {}: not found.", instance.instance()));
        exit(2);
    }
}

fn support_ansi_escape(fd: i32) -> bool {
    match isatty(fd) {
        Ok(b) => {
            if b && env!("TERM") != "dumb" {
                return true;
            } else {
                return false;
            }
        },
        Err(_) => return false
    }
}

pub fn print_warning(message: String) {
    if support_ansi_escape(2) {
        eprintln!("[1m[93mwarning:[0m {}", &message);
    } else {
        eprintln!("WARNING: {}", &message);
    }
} 

pub fn print_error(message: String) {
    if support_ansi_escape(2) {
        eprintln!("[1m[31merror:[0m {}", &message);
    } else {
        eprintln!("ERROR: {}", &message);
    }
} 


pub fn print_help_msg(args: &str) {
    println!("pacwrap error: {} ", args);
    println!("Try 'pacwrap -h' for more information on valid operational parameters.");
    exit(1);
}
