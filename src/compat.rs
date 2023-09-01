use std::rc::Rc;

use alpm::PackageReason;

use crate::config::{self, InstanceType};
use crate::config::InstanceHandle;
use crate::log::Logger;
use crate::sync;
use crate::utils::print_error;
use crate::Arguments;
use crate::config::{Instance, InsVars};
use crate::utils::env_var;

fn save_bash_configuration(ins: &str) {
    let mut logger = Logger::new("pacwrap-compat").init().unwrap();
    let mut pkgs = Vec::new();
    let deps: Vec<Rc<str>> = env_var("PACWRAP_DEPS").split_whitespace().map(|a| a.into()).collect(); 
    let ctype = InstanceType::new(env_var("PACWRAP_TYPE").as_str()); 
    let mut instance = match config::provide_some_handle(ins) {
        Some(handle) => handle,
        None => {
            let vars = InsVars::new(ins);
            let cfg = Instance::new(ctype, pkgs.clone(), deps.clone());
            InstanceHandle::new(cfg, vars)
        }
    };

    let depends = instance.metadata().dependencies();
    let dep_depth = depends.len();
    let alpm =  sync::instantiate_alpm(&instance);
    let mut skip = Vec::new();

    if dep_depth > 0 {
        let dep = &depends[dep_depth-1];

        if let Some(dep_instance) = config::provide_some_handle(dep) {
            let alpm =  sync::instantiate_alpm(&dep_instance);

            for pkg in alpm.localdb()
                .pkgs()
                .iter()
                .filter(|p| p.reason() == PackageReason::Explicit)
                .collect::<Vec<_>>() {
                skip.push(pkg.name().to_string());
            }
        }
    }

    for pkg in alpm.localdb()
        .pkgs()
        .iter()
        .filter(|p| p.reason() == PackageReason::Explicit 
            && ! skip
                .contains(&p.name()
                    .into()))
        .collect::<Vec<_>>() {
        pkgs.push(pkg.name().into());
    }

    alpm.release().unwrap();
    instance.metadata_mut().set(deps, pkgs);
    config::save_handle(&instance).ok(); 
    logger.log(format!("Configuration file written for {} by request of compatibility layer.", instance.vars().instance())).unwrap();
}

fn bash_configuration(instance: &str) {
    let ins = &config::provide_handle(instance);
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
}

pub fn compat() {
    let mut save = false;
    let mut bash = false;
    let mut test = false;

    let args = Arguments::new()
        .prefix("-Axc")
        .switch("-s", "--save", &mut save)
        .switch("-l", "--link", &mut bash)
        .switch("-t", "--test", &mut test) 
        .parse_arguments();
    let mut runtime = args.get_runtime().clone();
    args.require_target(1);

    let instance = runtime.remove(0);    

    if save { save_bash_configuration(&instance); }
    else if bash { bash_configuration(&instance); }
    else { print_error(format!("Invalid switch sequence.")) }  
}

