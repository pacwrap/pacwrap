use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::*};
use crate::config::permission::Condition::Success;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NONE;

#[typetag::serde]
impl Permission for NONE {
    fn check(&self) -> Result<Option<Condition>, PermError> { Ok(Some(Success)) }
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {}
    fn module(&self) -> &str { "NONE" }
}
