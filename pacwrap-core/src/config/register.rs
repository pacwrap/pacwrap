use std::vec::Vec;

use crate::{exec::args::ExecutionArgs, 
    config::{InsVars,
        Permission, 
        Dbus, 
        permission::*,
        filesystem::{Filesystem, BindError}},
    utils::print_warning, 
    error::*,
    err};

use super::ConfigError;

pub fn register_filesystems(per: &Vec<Box<dyn Filesystem>>, vars: &InsVars, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
       match p.check(vars) {
            Ok(_) => p.register(args, vars),
            Err(condition) => match condition {
                BindError::Warn(_) => print_warning(ConfigError::Filesystem(p.module(), condition)),
                BindError::Fail(_) => err!(ConfigError::Filesystem(p.module(), condition))?
            }
        }
    }

    Ok(())
}

pub fn register_permissions(per: &Vec<Box<dyn Permission>>, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        match p.check() {
            Ok(condition) => match condition {
                Some(b) => {
                    p.register(args);
                    
                    if let Condition::SuccessWarn(warning) = b {
                        print_warning(format!("{}: {} ", p.module(), warning));
                    }
                },
                None => continue
            },
            Err(condition) => match condition {
                PermError::Warn(_) => print_warning(ConfigError::Permission(p.module(), condition)),
                PermError::Fail(_) => err!(ConfigError::Permission(p.module(), condition))?
            }
        }    
    }

    Ok(())
}

pub fn register_dbus(per: &Vec<Box<dyn Dbus>>, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        p.register(args);
    }

    Ok(())
}
