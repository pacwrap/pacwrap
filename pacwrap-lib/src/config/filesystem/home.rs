use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;
use crate::config::filesystem::{Filesystem, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HOME;

#[typetag::serde]
impl Filesystem for HOME {
    fn check(&self, vars: &InsVars) -> Result<(), Error> {
        if ! Path::new(vars.home().as_ref()).exists() {
            Err(Error::new("HOME", format!("INSTANCE_HOME not found."), true))?
        }
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        args.bind(vars.home().as_ref(), vars.home_mount().as_ref());
        args.env("HOME", vars.home_mount().as_ref());
        args.env("USER", vars.user().as_ref());   
    }
}
