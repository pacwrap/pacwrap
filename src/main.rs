use std::{process::Command, env};
use utils::arguments::{self, Arguments};
use utils::handle_process;

mod config;
mod exec;
mod constants;
mod utils;
mod compat;
mod sync;
mod log;

#[derive(Clone, Copy)]
enum Options {
    Sync,
    Remove,
    Query,
    Compat,
    Interpose,
    Exec,
    BashCreate,
    BashProc,
    BashHelp,
    BashUtils,
    Version,
    None,
}

fn main() {
    let mut option: Options = Options::None;

    Arguments::new()
        .map(&mut option)
        .switch("-Q", "--query").set(Options::Query)
        .switch_big("--fake-chroot").set(Options::Sync)
        .switch("-S", "--sync").set(Options::Interpose)
        .switch("-R", "--remove").set(Options::Remove)
        .switch("-E", "--exec").set(Options::Exec)
        .switch("-V", "--version").set(Options::Version)
        .switch("-Axc", "--aux-compat").set(Options::Compat)
        .switch("-C", "--create").set(Options::BashCreate)
        .switch("-P", "--proc").set(Options::BashProc)
        .switch("-h", "--man").set(Options::BashHelp)
        .switch("-U", "--utils").set(Options::BashUtils)
        .parse_arguments();

    match option {
        Options::Exec => exec::execute(),
        Options::Sync => sync::execute(), 
        Options::Interpose => interpose(),  
        Options::Query => sync::query(), 
        Options::Remove => sync::remove(),
        Options::Compat => compat::compat(),
        Options::Version => print_version(),
        Options::BashUtils => execute_pacwrap_bash("pacwrap-utils"),
        Options::BashCreate => execute_pacwrap_bash("pacwrap-create"),
        Options::BashProc => execute_pacwrap_bash("pacwrap-ps"), 
        Options::BashHelp => execute_pacwrap_bash("pacwrap-man"),
        Options::None => arguments::invalid(),
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
