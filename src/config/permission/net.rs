use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::*};
use crate::config::permission::Condition::Success;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NET;

#[typetag::serde]
impl Permission for NET {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        args.push_env("--share-net");
        args.bind("/etc/resolv.conf", "/etc/resolv.conf");
    }

    fn module(&self) -> &str {
        "NET"
    }
}
