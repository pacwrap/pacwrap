use std::env::var;

use crate::constants::{LOCATION, HOME, USER};
use crate::config::instance::Instance;

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

        let mut vars = Self {
            home: format!("{}/home/{}", LOCATION.get_data(), ins),
            root: format!("{}/root/{}", LOCATION.get_data(), ins),
            pacman_gnupg: format!("{}/pacman/gnupg", LOCATION.get_data()),
            pacman_sync: format!("{}/pacman/sync", LOCATION.get_data()),
            pacman_cache: format!("{}/pkg", LOCATION.get_cache()),
            pacman_mirrorlist: format!("{}/pacman.d/mirrorlist", LOCATION.get_config()),
            sync: format!("{}/pacman/sync/pacman.{}.conf", LOCATION.get_config(), ins),
            syncdb: format!("{}/pacman/syncdb/pacman.{}.conf", LOCATION.get_config(), ins), 
            config: format!("{}/instance/{}.yml", LOCATION.get_config(), ins), 
            home_mount: format!("/home/{}", &ins),   
            user: ins.clone(),
            instance: ins
        };

        if let Ok(var) = var("PACWRAP_HOME") { vars.home=var; }

        vars
    }

    pub fn debug(&self, cfg: &Instance, switch: &String, runtime: &Vec<String>) {
        print!("Arguments: "); for arg in runtime.iter() { print!("{} ", arg); } println!();
        println!("Switch: -{}", switch);
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
