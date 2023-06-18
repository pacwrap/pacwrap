use serde::{Deserialize, Serialize};

use std::env;

use crate::exec::args::ExecutionArgs;
use crate::utils::print_warning;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ENV {
    var: String,
    #[serde(default = "default_set")]  
    set: String
}

#[typetag::serde]
impl Permission for ENV {

    fn check(&self) -> Result<(),Error> {
        Ok(())
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        let mut set = self.set.to_owned();
        if set == "" { 
            match env::var(&self.var) { 
                Ok(env) => set = env, 
                Err(_) => { print_warning(format!("Environment variable {} is empty.", &self.var)) 
                } 
            }
        }
        args.env(&self.var, set);
    }
}

fn default_set() -> String {
    "".into()
}

