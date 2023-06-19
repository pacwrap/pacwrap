use serde::{Deserialize, Serialize};

use std::env;

use crate::exec::args::ExecutionArgs;
use crate::utils::print_warning;
use crate::config::{InsVars, Permission, permission::Error};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ENV { 
    #[serde(default)] 
    var: String,
    #[serde(default)]
    set: String,
    #[serde(default)]    
    variables: Vec<Var>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Var {
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
        if self.var != "" {
            let set = set_env(&self.var, &self.set);
            args.env(&self.var, set);         
        }

        for v in self.variables.iter() {
            let set = set_env(&v.var, &v.set);
            args.env(&v.var, set);
        }
    }
}

fn set_env(var: &String, set: &String) -> String {
     if set != "" { return set.to_owned(); }
    
     match env::var(&var) { 
        Ok(env) => env, 
        Err(_) => {
            print_warning(format!("Environment variable {} is empty.", var));
            String::new()
        }
    } 
}

fn default_set() -> String {
    "".into()
}

