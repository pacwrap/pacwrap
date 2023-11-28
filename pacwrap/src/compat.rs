use std::process::Command;

use pacwrap_core::{config, 
    utils::{handle_process, 
        arguments::{Arguments, Operand}}, ErrorKind}; 

fn save_configuration() -> Result<(), ErrorKind> {
    Err(ErrorKind::Message("This function has been deprecated."))?
}

fn print_configuration(instance: &str) -> Result<(),ErrorKind> {
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

pub fn compat(args: &mut Arguments) -> Result<(), ErrorKind> {
    match args.next().unwrap_or_default() {
        Operand::Short('s') | Operand::Long("save") => save_configuration(),
        Operand::Short('l') | Operand::Long("load") => print_configuration(args.target()?),
        _ => Err(args.invalid_operand())
    }
}

pub fn execute_bash(executable: &str, args: &mut Arguments) -> Result<(), ErrorKind> { 
    handle_process(&executable, Command::new(&executable)
        .args(args.values())
        .spawn())
}
