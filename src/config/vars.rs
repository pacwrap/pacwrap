use std::env::var;

use crate::constants::{PACWRAP_DATA_DIR, PACWRAP_CACHE_DIR, PACWRAP_CONFIG_DIR};
use crate::config::Instance;

pub struct LocationVars {
    pub data: String,
    pub cache: String,
    pub conf: String,
    pub home: String
}

impl LocationVars {
    pub fn new() -> Self {
        let home_dir = var("HOME").unwrap(); 

        let mut dir = Self {
            data: format!("{}{}", &home_dir, PACWRAP_DATA_DIR),
            cache: format!("{}{}", &home_dir, PACWRAP_CACHE_DIR),
            conf: format!("{}{}", &home_dir, PACWRAP_CONFIG_DIR),
            home: home_dir
        };

        if let Ok(var) = var("PACWRAP_DATA_DIR") { dir.data=var; }
        if let Ok(var) = var("PACWRAP_CACHE_DIR") { dir.cache=var; }
        if let Ok(var) = var("PACWRAP_CONFIG_DIR") { dir.conf=var; }
    
        dir
    }
}

#[derive(Clone)]
pub struct InsVars {
    home: String,
    root: String,
    user: String,
    config: String,
    instance: String,
    home_mount: String,
    pub pacman_sync: String,
    pub pacman_cache: String,
    pub pacman_gnupg: String,
    pub pacman_mirrorlist: String, 
    sync: String,
    syncdb: String 
}

impl InsVars {
    pub fn new(_i: impl Into<String>) -> Self {
        let ins = _i.into();
        let dir = LocationVars::new();
    
        let mut vars = Self {
            home: format!("{}/home/{}", dir.data, ins),
            root: format!("{}/root/{}", dir.data, ins),
            pacman_gnupg: format!("{}/pacman/gnupg", dir.data),
            pacman_sync: format!("{}/pacman/sync", dir.data),
            pacman_cache: format!("{}/pkg", dir.cache),
            pacman_mirrorlist: format!("{}/pacman.d/mirrorlist", dir.conf),
            sync: format!("{}/pacman/sync/pacman.{}.conf", dir.conf, ins),
            syncdb: format!("{}/pacman/syncdb/pacman.{}.conf", dir.conf, ins), 
            config: format!("{}/instance/{}.yml", dir.conf, ins), 
            home_mount: format!("/home/{}", ins),   
            user: ins.clone(),
            instance: ins.clone(),
        };

        if let Ok(var) = var("PACWRAP_HOME") { vars.home=var; }

        vars
    }

    pub fn debug(&self, cfg: &Instance, switch: &String, runtime: &Vec<String>) {
        print!("Arguments: "); for arg in runtime.iter() { print!("{} ", arg); } println!();
        println!("Switch: -{}", switch);
        println!("Instance: {}", self.instance);
        println!("User: {}", var("USER").unwrap());
        println!("Home: {}", var("HOME").unwrap());
        println!("allow_forking: {}", cfg.allow_forking());
        println!("retain_session: {}", cfg.retain_session());
        println!("Config: {}", self.config);      
        println!("INSTANCE_USER: {}", self.user);     
        println!("INSTANCE_ROOT: {}", self.root);   
        println!("INSTANCE_HOME: {}", self.home);
        println!("INSTANCE_HOME_MOUNT: {}", self.home_mount);
        println!("INSTANCE_SYNC: {}", self.sync);
        println!("INSTANCE_SYNCDB: {}", self.syncdb);
    }

    pub fn sync(&self) -> &String { &self.sync }
    pub fn syncdb(&self) -> &String { &self.syncdb }
    pub fn config_path(&self) -> &String { &self.config }
    pub fn root(&self) -> &String { &self.root }
    pub fn home(&self) -> &String { &self.home }
    pub fn home_mount(&self) -> &String { &self.home_mount }
    pub fn user(&self) -> &String { &self.user }
    pub fn instance(&self) -> &String { &self.instance }
}
