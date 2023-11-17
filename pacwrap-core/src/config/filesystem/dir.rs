#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NEW_DIR {
    #[serde(default)]
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for NEW_DIR {
    fn check(&self, _vars: &InsVars) -> Result<(), Error> {
        if self.path.len() == 0 {
            Err(Error::new("DIR", format!("Path not specified."), false))?
        }
       
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _vars: &InsVars) {
        for dir in self.path.iter() {
            args.dir(dir);
        }
    }
}
