#![allow(non_camel_case_types)]
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error, default_permission};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TO_ROOT {
    #[serde(default = "default_permission")]  
    permission: String,
    #[serde(default)] 
    path: Vec<String>
}
#[typetag::serde]
impl Filesystem for TO_ROOT {

    fn check(&self, vars: &InsVars) -> Result<(), Error> {
        let per = self.permission.to_lowercase();

        if per != "ro" && per != "rw" {
            Err(Error::new("TO_ROOT", format!("{} is an invalid permission.", self.permission), true))? 
        }

        if self.path.len() == 0 {
            Err(Error::new("TO_ROOT", format!("Path not specified."), false))?
        }

        if ! Path::new(&self.path[0]).exists() {
            Err(Error::new("TO_ROOT", format!("Source path not found."), true))?
        }
       
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        let src = &self.path[0];
        let mut dest: &String = src; 

        if self.path.len() > 1 { dest = &self.path[1]; }
 
        match self.permission.to_lowercase().as_str() {
            p if p == "rw" => args.bind(src, dest), 
            &_ =>  args.robind(src, dest)
        }
    }
}
