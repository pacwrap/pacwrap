use std::env::var;
use std::rc::Rc;
use std::sync::Arc;

use crate::constants::{LOCATION, HOME, USER};
use crate::config::instance::InstanceRuntime;

#[derive(Clone)]
pub struct InsVars {
    home: Rc<str>,
    root: Arc<str>,
    user: Rc<str>,
    config: Rc<str>,
    instance: Rc<str>,
    home_mount: Rc<str>,
    pub pacman_cache: Rc<str>,
    pub pacman_gnupg: Rc<str>,
}

impl InsVars {
    pub fn new(_i: impl Into<Rc<str>>) -> Self {
        let ins = _i.into();

        let mut vars = Self {
            home: format!("{}/home/{}", LOCATION.get_data(), ins).into(),
            root: format!("{}/root/{}", LOCATION.get_data(), ins).into(),
            pacman_gnupg: format!("{}/pacman/gnupg", LOCATION.get_data()).into(),
            pacman_cache: format!("{}/pkg", LOCATION.get_cache()).into(),
            config: format!("{}/instance/{}.yml", LOCATION.get_config(), ins).into(), 
            home_mount: format!("/home/{}", &ins).into(),   
            user: ins.clone(),
            instance: ins.into(),
        };

        if let Ok(var) = var("PACWRAP_HOME") { 
            vars.home=var.into(); 
        }

        vars
    }

    pub fn debug(&self, cfg: &InstanceRuntime, runtime: &Vec<Rc<str>>) { 
        let mut args = String::new();
        for arg in runtime.iter() {
            args.push_str(format!("{} ", arg).as_str()); 
        }

        println!("Arguments: {}", args);
        println!("Instance: {}", self.instance);
        println!("User: {}", *USER);
        println!("Home: {}", *HOME);
        println!("allow_forking: {}", cfg.allow_forking());
        println!("retain_session: {}", cfg.retain_session());
        println!("Config: {}", self.config);      
        println!("INSTANCE_USER: {}", self.user);     
        println!("INSTANCE_ROOT: {}", self.root);   
        println!("INSTANCE_HOME: {}", self.home);
        println!("INSTANCE_HOME_MOUNT: {}", self.home_mount);
    }

    pub fn config_path(&self) -> &Rc<str> { &self.config }
    pub fn root(&self) -> &Arc<str> { &self.root }
    pub fn home(&self) -> &Rc<str> { &self.home }
    pub fn home_mount(&self) -> &Rc<str> { &self.home_mount }
    pub fn user(&self) -> &Rc<str> { &self.user }
    pub fn instance(&self) -> &Rc<str> { &self.instance }
}
