use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NONE;

#[typetag::serde]
impl Permission for NONE {

    fn check(&self) -> Result<(),Error> {
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {}
}
