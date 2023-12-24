use std::borrow::Cow;
use std::env::var;
use std::fmt::{Debug, Formatter};

use crate::constants::{CACHE_DIR, CONFIG_DIR, DATA_DIR};

#[derive(Clone)]
pub struct InsVars<'a> {
    home: Cow<'a, str>,
    root: Cow<'a, str>,
    user: Cow<'a, str>,
    config: Cow<'a, str>,
    instance: Cow<'a, str>,
    home_mount: Cow<'a, str>,
    pacman_cache: Cow<'a, str>,
    pacman_gnupg: Cow<'a, str>,
}

impl <'a>InsVars<'a> {
    pub fn new(ins: &'a str) -> Self {
        Self {
            home: match var("PACWRAP_HOME") { 
               Err(_) => format!("{}/home/{ins}", *DATA_DIR), Ok(var) => var 
            }.into(),
            root: format!("{}/root/{ins}", *DATA_DIR).into(),
            pacman_gnupg: format!("{}/pacman/gnupg", *DATA_DIR).into(),
            pacman_cache: format!("{}/pkg", *CACHE_DIR).into(),
            config: format!("{}/instance/{ins}.yml", *CONFIG_DIR).into(), 
            home_mount: format!("/home/{ins}").into(),   
            user: ins.into(),
            instance: ins.into(),
        }
    }

    pub fn pacman_cache(&self) -> &str { 
        &self.pacman_cache 
    }

    pub fn pacman_gnupg(&self) -> &str { 
        &self.pacman_gnupg 
    }

    pub fn config_path(&self) -> &str { 
        &self.config 
    }

    pub fn root(&self) -> &str { 
        &self.root 
    }

    pub fn home(&self) -> &str { 
        &self.home 
    }

    pub fn home_mount(&self) -> &str { 
        &self.home_mount 
    }

    pub fn user(&self) -> &str { 
        &self.user 
    }

    pub fn instance(&self) -> &str { 
        &self.instance 
    }
}

impl <'a>Debug for InsVars<'a> {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "Instance:            {}", self.instance)?;
        writeln!(fmter, "Instance User:       {}", self.user)?;
        writeln!(fmter, "Instance Config:     {}", self.config)?;      
        writeln!(fmter, "Instance Root:       {}", self.root)?;   
        writeln!(fmter, "Instance Home:       {} -> {}", self.home, self.home_mount)
    }
}