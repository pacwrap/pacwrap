use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs, 
    config::InsVars, 
    config::filesystem::{Filesystem, BindError}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HOME;

#[typetag::serde]
impl Filesystem for HOME {
    fn check(&self, vars: &InsVars) -> Result<(), BindError> {
        if ! Path::new(vars.home()).exists() {
            Err(BindError::Fail(format!("INSTANCE_HOME not found.")))?
        }
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        args.bind(vars.home(), vars.home_mount());
        args.env("HOME", vars.home_mount());
        args.env("USER", vars.user());   
    }

    fn module(&self) -> &'static str {
        "HOME"
    }
}
