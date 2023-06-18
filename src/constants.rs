use std::env::var;

pub const BWRAP_EXECUTABLE: &str = "bwrap";
pub const PACWRAP_CONFIG_DIR: &str = "/.config/pacwrap";
pub const PACWRAP_DATA_DIR: &str = "/.local/share/pacwrap";
pub const PACWRAP_CACHE_DIR: &str = "/.cache/pacwrap";

pub fn return_home() -> String { var("HOME").unwrap() }
