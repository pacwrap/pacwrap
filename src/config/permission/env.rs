use serde::{Deserialize, Serialize};

use std::env;

use crate::exec::args::ExecutionArgs;
use crate::utils::print_warning;
use crate::config::{InsVars, Permission, permission::*, permission::Condition::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ENV { 
    #[serde(skip_serializing_if = "String::is_empty", default)] 
    var: String,
    #[serde(skip_serializing_if = "String::is_empty", default)] 
    set: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)] 
    variables: Vec<Var>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Var {
    var: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]   
    set: String
}

#[typetag::serde]
impl Permission for ENV {

    fn check(&self) -> Result<Option<Condition>, PermError> {
        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {        
        if self.var != "" {
            let set = set_env(&self.var, &self.set);
            args.env(&self.var, set);         
        }

        for v in self.variables.iter() {
            let set = set_env(&v.var, &v.set);
            args.env(&v.var, set);
        }
    }

    fn module(&self) -> &str {
        "ENV"
    }
}

fn set_env(var: &String, set: &String) -> String {
     if set != "" { return set.to_owned(); }
    
     match env::var(&var) { 
        Ok(env) => env, 
        Err(_) => {
            print_warning(format!("Environment variable {} is unset.", var));
            String::new()
        }
    } 
}
