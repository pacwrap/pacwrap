#![allow(non_camel_case_types)]

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::constants::HOME;
use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, BindError, default_permission, is_default_permission};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TO_HOME {
    #[serde(skip_serializing_if = "is_default_permission", default = "default_permission")]
    permission: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    path: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]  
    filesystem: Vec<Mount>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mount {
    #[serde(skip_serializing_if = "is_default_permission", default = "default_permission")] 
    permission: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for TO_HOME {
    fn check(&self, _vars: &InsVars) -> Result<(), BindError> {
        if self.path.len() > 0 {
            if let Err(e) = check_mount(&self.permission, &self.path[0]) {
                return Err(e);
            }
        } else {
            if self.filesystem.len() == 0 {
                Err(BindError::Warn(format!("Filesystem paths are undeclared.")))?
            }
        }

        for m in self.filesystem.iter() { 
            if m.path.len() == 0 {
                Err(BindError::Warn(format!("Filesystem paths are undeclared.")))? 
            }

            if let Err(e) = check_mount(&m.permission, &m.path[0]) {
                return Err(e);
            }
        }

        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        if self.path.len() > 0 { 
            bind_filesystem(args,vars, &self.permission, &self.path);
        }

        for m in self.filesystem.iter() { 
            bind_filesystem(args,vars, &m.permission, &m.path);
        }
    }

    fn module(&self) -> &'static str {
        "TO_HOME"
    }
}

fn bind_filesystem(args: &mut ExecutionArgs, vars: &InsVars, permission: &str, path: &Vec<String>) {
    let src = &path[0];
    let mut dest: &String = src; 

    if path.len() > 1 {
            dest = &path[1];
    }
  
    let path_src = format!("{}/{}", *HOME, path[0]);
    let path_dest = format!("{}/{}", vars.home_mount(), dest);

    match permission == "rw" {
        false => args.robind(path_src, path_dest),
        true => args.bind(path_src, path_dest),
    }
}

fn check_mount(permission: &String, path: &String) -> Result<(), BindError> {
    let per = permission.to_lowercase(); 
        
    if per != "ro" && per != "rw" {
        Err(BindError::Fail(format!("{} is an invalid permission.", permission)))?
    }

    if ! Path::new(&format!("{}/{}", *HOME, &path)).exists() {
        Err(BindError::Fail(format!("~/{} not found.", path)))?
    }

    Ok(())
}
