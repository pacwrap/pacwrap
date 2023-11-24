use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROOT;

#[typetag::serde]
impl Filesystem for ROOT {
    fn check(&self, vars: &InsVars) -> Result<(), Error> {
        if ! Path::new(vars.root()).exists() {
            Err(Error::new("ROOT", format!("Container {} not found. ", vars.instance()), true))?
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
}
