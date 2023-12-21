use std::{fmt::Display, 
    io::Write, 
    fmt::Formatter, 
    path::Path, 
    fs::File};

use crate::{ErrorKind, constants::{BOLD, RESET}};
use self::{filesystem::BindError, permission::PermError};

pub use self::{cache::InstanceCache, 
    instance::{Instance,
        InstanceHandle, 
        InstanceType}, 
    vars::InsVars, 
    filesystem::Filesystem, 
    permission::Permission, 
    dbus::Dbus};

pub mod vars;
pub mod filesystem;
pub mod permission;
pub mod dbus;
pub mod cache;
pub mod instance;
pub mod init;
pub mod register;

#[derive(Debug, Clone)]
pub enum ConfigError {
    Permission(&'static str, PermError),
    Filesystem(&'static str, BindError),
    Save(String, String),
    Load(String, String),
    AlreadyExists(String),
}

impl Display for ConfigError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::Filesystem(module, err) => write!(fmter, "Failed to register filesystem {}: {} ", module, err), 
            Self::Permission(module, err) => write!(fmter, "Failed to register permission {}: {} ", module, err),
            Self::Load(ins, error) => write!(fmter, "Failed to load '{ins}.yml': {error}"),
            Self::Save(ins, error) => write!(fmter, "Failed to save '{ins}.yml': {error}"),
            Self::AlreadyExists(ins) => write!(fmter, "Container {}{ins}{} already exists.", *BOLD, *RESET),
        }
    }
}

impl From<ConfigError> for String {
    fn from(value: ConfigError) -> Self {
        value.into()
    }
}

pub fn save_handle(ins: &InstanceHandle) -> Result<(), ErrorKind> {   
    let mut f = match File::create(Path::new(ins.vars().config_path())) {
        Ok(f) => f,
        Err(error) => Err(ErrorKind::IOError(ins.vars().config_path().into(), error.kind()))?
    };
    let config = match serde_yaml::to_string(&ins.instance()) {
        Ok(file) => file,
        Err(error) => Err(ErrorKind::Config(ConfigError::Save(ins.vars().instance().into(), error.to_string())))? 
    };
    
    match write!(f, "{}", config) {
        Ok(_) => Ok(()),
        Err(error) => Err(ErrorKind::IOError(ins.vars().config_path().into(), error.kind())) 
    }
}

#[inline]
pub fn provide_handle(instance: &str) -> Result<InstanceHandle, ErrorKind> { 
    let vars = InsVars::new(instance);

    if ! Path::new(vars.root()).exists() {  
        Err(ErrorKind::InstanceNotFound(instance.into()))?
    }

    handle(instance, vars)
}

#[inline]
pub fn provide_new_handle(instance: &str) -> Result<InstanceHandle, ErrorKind> {
    handle(instance, InsVars::new(instance))
}

fn handle<'a>(instance: &str, vars: InsVars<'a>) -> Result<InstanceHandle<'a>, ErrorKind> {
    match File::open(vars.config_path()) {
        Ok(file) => {
            let config = match serde_yaml::from_reader(&file) {
                Ok(file) => file,
                Err(error) => Err(ErrorKind::Config(ConfigError::Load(vars.instance().into(), error.to_string())))?
            };

            Ok(InstanceHandle::new(config, vars))
        },
        Err(error) => {
            let path = match error.kind() {
                std::io::ErrorKind::NotFound => format!("{instance}.yml"), _ => vars.config_path().into(),
            };

            Err(ErrorKind::IOError(path, error.kind()))
        }
    }
}
