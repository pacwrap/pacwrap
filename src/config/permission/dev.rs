use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{Permission, permission::*};
use crate::config::permission::{Condition::Success, PermError::Fail};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DEV {
    device: String
}

#[typetag::serde]
impl Permission for DEV {
    fn check(&self) -> Result<Option<Condition>, PermError> {  
        if ! Path::new(&format!("/dev/{}",self.device)).exists() {
            Err(Fail(format!("/dev/{} is inaccessible.", self.device)))?
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs) { 
        args.dev(&format!("/dev/{}", self.device));
    }

    fn module(&self) -> &str {
        "DEV"
    }
}
