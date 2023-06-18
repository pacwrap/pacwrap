use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DEV {
    device: String
}

#[typetag::serde]
impl Permission for DEV {
    fn check(&self) -> Result<(),Error> {  
         if ! Path::new(&format!("/dev/{}",self.device)).exists() {
            Err(Error::new("dev", format!("/dev/{} is inaccessible.", self.device)))?
        }

        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) { 
        args.dev(&format!("/dev/{}", self.device));
    }
}
