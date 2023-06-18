use std::env;
use std::process::Command;
use std::collections::HashMap;
use utils::arguments::Arguments;
use utils::print_help_msg;

mod config;
mod exec;
mod constants;
mod utils;

fn main() {
    let args = Arguments::new(0, "-", HashMap::from([("--exec".into(), "E".into()),
                                                     ("--explicit".into(), "E".into()),
                                                     ("--pacman".into(),"E".into()),
                                                     ("--gen-cfg".into(),"Axc".into()), 
                                                     ("--version".into(),"V".into()),  
                                                     ("--create".into(),"C".into()), 
                                                     ("--sync".into(),"S".into()),
                                                     ("--help".into(),"h".into()),
                                                     ("--utils".into(),"U".into()),
                                                     ("--process".into(),"P".into()),]));
    
    match args.get_switch().as_str() {
        s if s.starts_with("E") => exec::execute(),
        s if s.starts_with("V") => print_version(), 
        s if s.starts_with("S") => execute_pacwrap_bash("pacwrap-sync".to_string()),
        s if s.starts_with("U") => execute_pacwrap_bash("pacwrap-utils".to_string()),
        s if s.starts_with("C") => execute_pacwrap_bash("pacwrap-create".to_string()),
        s if s.starts_with("P") => execute_pacwrap_bash("pacwrap-ps".to_string()),
        s if s.starts_with("v") => execute_pacwrap_bash("pacwrap-man".to_string()), 
        s if s.starts_with("h") => execute_pacwrap_bash("pacwrap-man".to_string()),
        s if s.starts_with("Axc") => config::aux_config(), 
        &_ => {
            let mut ar = String::new();
            for arg in env::args().skip(1).collect::<Vec<_>>().iter() {
                ar.push_str(&format!("{} ", &arg));
            }
            print_help_msg(&format!("Invalid arguments -- {}", ar));
        } 
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

fn execute_pacwrap_bash(executable: String) { 
        let mut process = Command::new(&executable)
        .args(env::args().skip(1).collect::<Vec<_>>())
        .spawn().expect("Command failed.");
        process.wait().expect("failed to wait on child");    
}
