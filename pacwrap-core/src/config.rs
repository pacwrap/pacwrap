use std::{fmt::Display, 
    io::{Write, ErrorKind::NotFound}, 
    fmt::Formatter, 
    path::Path, 
    fs::File};

use crate::{err, impl_error, ErrorKind, error::*, constants::{BOLD, RESET}};
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
    ConfigNotFound(String),
}

impl_error!(ConfigError);

impl Display for ConfigError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::Filesystem(module, err) => write!(fmter, "Failed to register filesystem {}: {} ", module, err), 
            Self::Permission(module, err) => write!(fmter, "Failed to register permission {}: {} ", module, err),
            Self::Load(ins, error) => write!(fmter, "Failed to load '{ins}.yml': {error}"),
            Self::Save(ins, error) => write!(fmter, "Failed to save '{ins}.yml': {error}"),
            Self::AlreadyExists(ins) => write!(fmter, "Container {}{ins}{} already exists.", *BOLD, *RESET),
            Self::ConfigNotFound(ins) => write!(fmter, "Configuration '{}{ins}{}.yml' not found.", *BOLD, *RESET)
        }
    }
}

impl From<ConfigError> for String {
    fn from(value: ConfigError) -> Self {
        value.into()
    }
}

pub fn save_handle(ins: &InstanceHandle) -> Result<()> {   
    let mut f = match File::create(Path::new(ins.vars().config_path())) {
        Ok(f) => f,
        Err(error) => err!(ErrorKind::IOError(ins.vars().config_path().into(), error.kind()))?
    };
    let config = match serde_yaml::to_string(&ins.instance()) {
        Ok(file) => file,
        Err(error) => err!(ConfigError::Save(ins.vars().instance().into(), error.to_string()))? 
    };
    
    match write!(f, "{}", config) {
        Ok(_) => Ok(()),
        Err(error) => err!(ErrorKind::IOError(ins.vars().config_path().into(), error.kind())) 
    }
}

#[inline]
pub fn provide_handle(instance: &str) -> Result<InstanceHandle> { 
    let vars = InsVars::new(instance);

    if ! Path::new(vars.root()).exists() {  
        err!(ErrorKind::InstanceNotFound(instance.into()))?
    }

    handle(instance, vars)
}

#[inline]
pub fn provide_new_handle(instance: &str) -> Result<InstanceHandle> {
    handle(instance, InsVars::new(instance))
}

fn handle<'a>(instance: &str, vars: InsVars<'a>) -> Result<InstanceHandle<'a>> {
    match File::open(vars.config_path()) {
        Ok(file) => {
            let config = match serde_yaml::from_reader(&file) {
                Ok(file) => file,
                Err(error) => err!(ConfigError::Load(vars.instance().into(), error.to_string()))?
            };

            Ok(InstanceHandle::new(config, vars))
        },
        Err(error) => match error.kind() {
            NotFound => err!(ConfigError::ConfigNotFound(instance.into()))?,
            _ => err!(ErrorKind::IOError(vars.config_path().into(), error.kind()))?,
        }
    }
}
