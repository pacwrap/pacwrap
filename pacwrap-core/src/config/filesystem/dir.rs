#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, BindError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NEW_DIR {
    #[serde(default)]
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for NEW_DIR {
    fn check(&self, _vars: &InsVars) -> Result<(), BindError> {
        if self.path.len() == 0 {
            Err(BindError::Fail(format!("Path not specified.")))?
        }
       
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, _vars: &InsVars) {
        for dir in self.path.iter() {
            args.dir(dir);
        }
    }

    fn module(&self) -> &'static str {
        "DIR"
    }
}
