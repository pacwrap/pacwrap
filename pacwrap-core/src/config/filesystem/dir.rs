use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs, 
    config::InsVars, 
    config::filesystem::{Filesystem, BindError}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIR {
    #[serde(default)]
    path: Vec<String>
}

#[typetag::serde]
impl Filesystem for DIR {
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
