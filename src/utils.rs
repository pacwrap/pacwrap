use std::path::Path;
use std::process::{Child, exit};
use std::env::var;
use std::os::unix::net::UnixStream;
use std::fmt::Display;

use console::style;
use serde_json::Value;

use crate::config::vars::InsVars;

pub use arguments::Arguments;
pub use termcontrol::TermControl;

pub mod termcontrol;
pub mod arguments;
pub mod prompt;

pub fn test_root(instance: &InsVars) {
    if ! Path::new(&instance.root()).exists() || ! Path::new(&instance.home()).exists() {  
        print_error(format!("Target container {}: not found.", instance.instance()));
        exit(2);
    }
}

pub fn print_warning(message: impl Into<String> + Display) {
    eprintln!("{} {}", style("warning:").bold().yellow(), &message);
} 

pub fn print_error(message: impl Into<String> + Display) {
    eprintln!("{} {}", style("error:").bold().red(), &message);
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

pub fn whitespace(total: usize, current: usize) -> String {
    let difference = total-current;
    let mut whitespace = String::new();
    if difference > 0 {
        for _ in 0..difference {
            whitespace.push_str(" ");
        } 
    }
    whitespace
}
