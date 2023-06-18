#![allow(non_camel_case_types)]

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::constants::return_home;
use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error, default_permission};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TO_HOME {
    #[serde(default = "default_permission")]  
    permission: String,
    #[serde(default)]
    path: Vec<String>
}
#[typetag::serde]
impl Filesystem for TO_HOME {
    fn check(&self, vars: &InsVars) -> Result<(), Error> {
        let per = self.permission.to_lowercase(); 
        
        if per != "ro" && per != "rw" {
            Err(Error::new("TO_HOME", format!("{} is an invalid permission.", self.permission), true))? 
        }

        if self.path.len() == 0 {
            Err(Error::new("TO_HOME", format!("Path not specified."), false))?
        }

        if ! Path::new(&format!("{}/{}", return_home(), &self.path[0])).exists() {
            Err(Error::new("TO_HOME", format!("~/{} not found.", self.path[0]), true))?
        }
       
        Ok(())
    }


    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        let src = &self.path[0];
        let mut dest: &String = src; 

        if self.path.len() > 1 { dest = &self.path[1]; }
  
        let path_src = format!("{}/{}", return_home(), &self.path[0]);
        let path_dest = format!("{}/{}", vars.home_mount(), dest);

        match self.permission.to_lowercase().as_str() {
            p if p == "rw" => args.bind(path_src, path_dest), 
            &_ => args.robind(path_src, path_dest)
        }
    }
}
