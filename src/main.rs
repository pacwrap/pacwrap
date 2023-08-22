use std::{process::Command, env};
use utils::arguments::{self, Arguments};
use utils::handle_process;

mod config;
mod exec;
mod constants;
mod utils;
mod compat;
mod sync;

fn main() {
    let mut sync = false;
    let mut sync_chroot = false; 
    let mut exec = false;
    let mut version = false;
    let mut compat = false;
    let mut query = false;
    let mut remove = false;

    let mut bash_create = false;
    let mut bash_help = false;
    let mut bash_utils = false;
    let mut bash_proc = false;
    
    Arguments::new() 
        .switch("-Q", "--query", &mut query) 
        .switch_big("--fake-chroot", &mut sync_chroot)
        .switch("-S", "--sync", &mut sync)
        .switch("-R", "--remove", &mut remove)
        .switch("-E", "--exec", &mut exec)
        .switch("-V", "--version", &mut version) 
        .switch("-Axc", "--aux-compat", &mut compat)
        .switch("-C", "--create", &mut bash_create)
        .switch("-P", "--proc", &mut bash_proc)
        .switch("-h", "--man", &mut bash_help)
        .switch("-U", "--utils", &mut bash_utils)
        .parse_arguments();

    if exec { exec::execute() }
    else if sync_chroot { sync::execute(); } 
    else if sync { interpose() }  
    else if query { sync::query(); } 
    else if remove { sync::remove(); }
    else if compat { compat::compat(); }
    else if version { print_version(); }
    else if bash_utils { execute_pacwrap_bash("pacwrap-utils"); }
    else if bash_create { execute_pacwrap_bash("pacwrap-create"); }
    else if bash_proc { execute_pacwrap_bash("pacwrap-ps"); }
    else if bash_help { execute_pacwrap_bash("pacwrap-man"); }
    else { arguments::invalid(); } 
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

fn interpose() {
    let arguments = env::args().skip(1).collect::<Vec<_>>(); 
    let all_args = env::args().collect::<Vec<_>>();
    let this_executable = all_args.first().unwrap();

    handle_process(Command::new(this_executable)
        .env("LD_PRELOAD", "/usr/lib/libfakeroot/fakechroot/libfakechroot.so")
        .arg("--fake-chroot")
        .args(arguments)
        .spawn());
}

fn execute_pacwrap_bash(executable: &str) { 
    handle_process(Command::new(&executable)
        .arg("")
        .args(env::args().skip(1).collect::<Vec<_>>())
        .spawn());
}
