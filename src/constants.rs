use std::env::var;
use std::process::id;

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
    pub static ref LOG_LOCATION: &'static str = format!("{}/pacwrap.log", LOCATION.get_data()).leak();
    pub static ref IS_COLOR_TERMINLAL: bool = crate::utils::is_color_terminal();
    pub static ref IS_TRUECOLOR_TERMINLAL: bool = crate::utils::is_truecolor_terminal();
    pub static ref BOLD: &'static str = bold();
    pub static ref RESET: &'static str = reset();
    pub static ref DIM: &'static str = dim();
    pub static ref BOLD_WHITE: &'static str = bold_white();
    pub static ref BOLD_YELLOW: &'static str = bold_yellow();
    pub static ref BOLD_RED: &'static str = bold_red();
    pub static ref BOLD_GREEN: &'static str = bold_green(); 
    pub static ref BAR_GREEN: &'static str = bar_green(); 
    pub static ref BAR_CYAN: &'static str = bar_cyan(); 
    pub static ref ARROW_CYAN: &'static str = arrow_cyan();  
    pub static ref ARROW_RED: &'static str = arrow_red(); 
    pub static ref ARROW_GREEN: &'static str = arrow_green(); 
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

fn arrow_red() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;31m->[0m" } else { "->" }
}

fn arrow_cyan() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;36m->[0m" } else { "->" }
}

fn arrow_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m->[0m" } else { "->" }
}

fn bar_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m::[0m" } else { "::" }
}

fn bar_cyan() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;36m::[0m" } else { "::" }
}

fn dim() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[2m" } else { "" }
}

fn bold() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1m" } else { "" }
}

fn bold_white() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;37m" } else { "" }
}

fn bold_red() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;31m" } else { "" }
}

fn bold_yellow() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;33m" } else { "" }
}

fn bold_green() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[1;32m" } else { "" }
}

fn reset() -> &'static str {
    if *IS_COLOR_TERMINLAL { "[0m" } else { "" }
}
