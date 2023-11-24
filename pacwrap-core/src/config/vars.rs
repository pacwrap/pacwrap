use std::env::var;

use crate::constants::{LOCATION, HOME, USER};
use crate::config::instance::InstanceRuntime;

#[derive(Clone)]
pub struct InsVars<'a> {
    home: &'a str,
    root: &'a str,
    user: &'a str,
    config: &'a str,
    instance: &'a str,
    home_mount: &'a str,
    pacman_cache: &'a str,
    pacman_gnupg: &'a str,
}

impl <'a>InsVars<'a> {
    pub fn new(ins: &'a str) -> Self {
        Self {
            home: match var("PACWRAP_HOME") { 
               Err(_) => format!("{}/home/{ins}", LOCATION.get_data()), Ok(var) => var 
            }.leak(),
            root: format!("{}/root/{ins}", LOCATION.get_data()).leak(),
            pacman_gnupg: format!("{}/pacman/gnupg", LOCATION.get_data()).leak(),
            pacman_cache: format!("{}/pkg", LOCATION.get_cache()).leak(),
            config: format!("{}/instance/{ins}.yml", LOCATION.get_config()).leak(), 
            home_mount: format!("/home/{ins}").leak(),   
            user: ins,
            instance: ins,
        }
    }

    pub fn debug(&self, cfg: &InstanceRuntime, runtime: &Vec<&str>) { 
        let mut args = String::new();

        for arg in runtime.iter() {
            args.push_str(&format!("{arg} "));
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

    pub fn pacman_cache(&self) -> &'a str { 
        &self.pacman_cache 
    }

    pub fn pacman_gnupg(&self) -> &'a str { 
        &self.pacman_gnupg 
    }

    pub fn config_path(&self) -> &'a str { 
        &self.config 
    }

    pub fn root(&self) -> &'a str { 
        &self.root 
    }

    pub fn home(&self) -> &str { 
        &self.home 
    }

    pub fn home_mount(&self) -> &'a str { 
        &self.home_mount 
    }

    pub fn user(&self) -> &'a str { 
        &self.user 
    }

    pub fn instance(&self) -> &'a str { 
        &self.instance 
    }
}
