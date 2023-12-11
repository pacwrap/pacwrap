use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs,
    config::{Permission, permission::*},
    config::permission::{Condition::Success, PermError::Fail}};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DEV {
    devices: Vec<String>
}

#[typetag::serde]
impl Permission for DEV {
    fn check(&self) -> Result<Option<Condition>, PermError> {   
        for device in self.devices.iter() {
            if ! Path::new(&format!("/dev/{}", device)).exists() {
                Err(Fail(format!("/dev/{} is inaccessible.", device)))?
            }
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs) { 
        for device in self.devices.iter() {
            args.dev(&format!("/dev/{}", device));
        }
    }

    fn module(&self) -> &'static str {
        "DEV"
    }
}
