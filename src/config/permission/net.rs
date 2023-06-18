use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NET;

#[typetag::serde]
impl Permission for NET {
    fn check(&self) -> Result<(),Error> {
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        args.push_env("--share-net");
        args.bind("/etc/resolv.conf", "/etc/resolv.conf");
    }
}
