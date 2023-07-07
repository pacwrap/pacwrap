use std::fs::File;

use crate::utils::print_error;
use crate::Arguments;
use crate::config::read_yaml;
use crate::config::save_configuration;
use crate::config::load_configuration;
use crate::config::{Instance, InsVars};
use crate::utils::env_var;

fn save_bash_configuration(ins: &String) {
    let pkgs: Vec<String> = env_var("PACWRAP_PKGS").split_whitespace().map(str::to_string).collect(); 
    let deps: Vec<String> = env_var("PACWRAP_DEPS").split_whitespace().map(str::to_string).collect();
    let ctype = env_var("PACWRAP_TYPE"); 
    let vars = InsVars::new(ins);
    let mut instance: Instance;
    let path: &str = vars.config_path().as_str();

    match File::open(path) {
        Ok(file) => instance = read_yaml(file),
        Err(_) => instance = Instance::new(ctype.clone(), pkgs.clone(), deps.clone()),
    }
    
    instance.set(ctype, deps, pkgs);
    save_configuration(&instance, path.to_string()); 
}

fn bash_configuration(instance: &String) {
    let vars = InsVars::new(instance);
    let ins = &load_configuration(vars.config_path());
    let depends = ins.dependencies();
    let pkgs = ins.explicit_packages();
    let mut pkgs_string = String::new();
    let mut depends_string = String::new();

    println!("INSTANCE_CONFIG[{},0]={}", instance, ins.container_type());   

    for i in depends.iter() {
        depends_string.push_str(&format!("{} ", i));    
    }
    println!("INSTANCE_CONFIG[{},1]=\"{}\"", instance, depends_string);

    for i in pkgs.iter() {
        pkgs_string.push_str(&format!("{} ", i));
    }
    println!("INSTANCE_CONFIG[{},3]=\"{}\"", instance, pkgs_string);
}

pub fn compat() {
    let mut save = false;
    let mut bash = false;

    let args = Arguments::new()
        .prefix("-Axc")
        .switch("-s", "--save", &mut save)
        .switch("-l", "--link", &mut bash)
        .parse_arguments();
    let mut runtime = args.get_runtime().clone();
    args.require_target(1);

    let instance = runtime.remove(0);    

    if save { save_bash_configuration(&instance); }
    else if bash { bash_configuration(&instance); }
    else { print_error(format!("Invalid switch sequence.")) }  
}

