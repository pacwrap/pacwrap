use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs, 
    config::InsVars, 
    config::filesystem::{Filesystem, BindError}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROOT;

#[typetag::serde]
impl Filesystem for ROOT {
    fn check(&self, vars: &InsVars) -> Result<(), BindError> {
        if ! Path::new(vars.root()).exists() {
            Err(BindError::Fail(format!("Container {} not found. ", vars.instance())))?
        }
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) { 
        args.robind(format!("{}/usr", vars.root()), "/usr");
        args.robind(format!("{}/etc", vars.root()), "/etc");
        args.symlink("/usr/lib", "/lib");
        args.symlink("/usr/lib", "/lib64");
        args.symlink("/usr/bin", "/bin");
        args.symlink("/usr/bin", "/sbin");
    }

    fn module(&self) -> &'static str {
        "ROOT"
    }
}
