use std::env::var;
use std::process::id;
use std::sync::Arc;

use lazy_static::lazy_static;
use nix::unistd::geteuid;

use crate::utils::env_var;

pub const BWRAP_EXECUTABLE: &str = "bwrap";

const PACWRAP_CONFIG_DIR: &str = "/.config/pacwrap";
const PACWRAP_DATA_DIR: &str = "/.local/share/pacwrap";
const PACWRAP_CACHE_DIR: &str = "/.cache/pacwrap";

lazy_static! {
   pub static ref LOCATION: LocationVars = LocationVars::new();
   pub static ref HOME: String = env_var("HOME");
   pub static ref USER: String = env_var("USER");
   pub static ref XDG_RUNTIME_DIR: String = format!("/run/user/{}/", geteuid());
   pub static ref DBUS_SOCKET: String = format!("/run/user/{}/pacwrap_dbus_{}", geteuid(), &id());
   pub static ref LOG_LOCATION: Arc<str> = format!("{}/pacwrap.log", LOCATION.get_data()).into();
}

pub struct LocationVars {
    data: String,
    cache: String,
    config: String,
}

impl LocationVars {
    pub fn new() -> Self {
        let mut dir = Self {
            data: format!("{}{}", *HOME, PACWRAP_DATA_DIR),
            cache: format!("{}{}", *HOME, PACWRAP_CACHE_DIR),
            config: format!("{}{}", *HOME, PACWRAP_CONFIG_DIR),
        };

        if let Ok(var) = var("PACWRAP_DATA_DIR") { dir.data=var; }
        if let Ok(var) = var("PACWRAP_CACHE_DIR") { dir.cache=var; }
        if let Ok(var) = var("PACWRAP_CONFIG_DIR") { dir.config=var; }
    
        dir
    }

    pub fn get_cache(&self) -> &String { &self.cache }
    pub fn get_data(&self) -> &String { &self.data }
    pub fn get_config(&self) -> &String { &self.config }
}
