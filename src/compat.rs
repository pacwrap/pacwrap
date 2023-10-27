use std::{process::Command, env};


use crate::config;
use crate::utils::print_help_error;
use crate::utils::{arguments::{Arguments, Operand}, 
    handle_process};


fn save_configuration() -> Result<(), String> {
    Err("This functionality has been deprecated and removed.")?
}

fn print_configuration(instance: &str) -> Result<(),String> {
    let ins = config::provide_new_handle(instance)?;
    let depends = ins.metadata().dependencies();
    let pkgs = ins.metadata().explicit_packages();
    let mut pkgs_string = String::new();
    let mut depends_string = String::new();

    println!("INSTANCE_CONFIG[{},0]={}", instance, ins.metadata().container_type());   

    for i in depends.iter() {
        depends_string.push_str(&format!("{} ", i));    
    }
    println!("INSTANCE_CONFIG[{},1]=\"{}\"", instance, depends_string);

    for i in pkgs.iter() {
        pkgs_string.push_str(&format!("{} ", i));
    }

    println!("INSTANCE_CONFIG[{},3]=\"{}\"", instance, pkgs_string);
    Ok(())
}

pub fn compat(mut args: Arguments) {
    let result = match args.next().unwrap_or_default() {
        Operand::Short('s') | Operand::Long("save") => save_configuration(),
        Operand::Short('l') | Operand::Long("load") => print_configuration(args.target()),
        _ => Err(args.invalid_operand())
    };

    if let Err(error) = result {
        print_help_error(error);
    }
}

pub fn execute_bash(executable: &str) { 
    handle_process(Command::new(format!("pacwrap-{executable}"))
        .arg("")
        .args(env::args().skip(1).collect::<Vec<_>>())
        .spawn());
}
