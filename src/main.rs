use std::env;
use std::process::Command;
use utils::arguments::Arguments;
use utils::print_help_msg;

mod config;
mod exec;
mod constants;
mod utils;
mod compat;
mod sync;

fn main() {
    let mut sync = false;
    let mut exec = false;
    let mut version = false;
    let mut compat = false;

    let mut bash_create = false;
    let mut bash_help = false;
    let mut bash_utils = false;
    let mut bash_proc = false;
    
    Arguments::new() 
        .switch("-S", "--sync", &mut sync)
        .switch("-E", "--exec", &mut exec)
        .switch("-V", "--version", &mut version) 
        .switch("-Axc", "--aux-compat", &mut compat)
        .switch("-C", "--create", &mut bash_create)
        .switch("-P", "--proc", &mut bash_proc)
        .switch("-h", "--man", &mut bash_help)
        .switch("-U", "--utils", &mut bash_utils)
        .parse_arguments();

    if exec { exec::execute() }
    else if sync { sync::execute(); }
    else if compat { compat::compat(); }
    else if version { print_version(); }
    else if bash_utils { execute_pacwrap_bash("pacwrap-utils"); }
    else if bash_create { execute_pacwrap_bash("pacwrap-create"); }
    else if bash_proc { execute_pacwrap_bash("pacwrap-ps"); }
    else if bash_help { execute_pacwrap_bash("pacwrap-man"); }
    else {
        let mut ar = String::new();
        for arg in env::args().skip(1).collect::<Vec<_>>().iter() {
            ar.push_str(&format!("{} ", &arg));
        } 
        ar.truncate(ar.len()-1);
        print_help_msg(&format!("Invalid arguments -- '{}'", ar));
    } 
}

fn print_version() {
    println!("{} {} ", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    let info=concat!("Copyright (C) 2023 Xavier R.M.\n\n",
                     "Website: https://git.sapphirus.org/pacwrap\n",
                     "Github: https://github.com/sapphirusberyl/pacwrap\n\n",
                     "This program may be freely redistributed under\n",
                     "the terms of the GNU General Public License v3.\n");
    print!("{}", info);
}

fn execute_pacwrap_bash(executable: &str) { 
        let mut process = Command::new(&executable)
        .args(env::args().skip(1).collect::<Vec<_>>())
        .spawn().expect("Command failed.");
        process.wait().expect("failed to wait on child");    
}
