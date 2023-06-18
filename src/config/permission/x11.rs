use std::path::Path;
use std::env::var;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct X11 {
    #[serde(skip_serializing_if = "is_default", default = "default_display")]
    display: i8,
}

#[typetag::serde]
impl Permission for X11 {
    fn check(&self) -> Result<(),Error> {  
        if ! Path::new(&format!("/tmp/.X11-unix/X{}", self.display)).exists() {
            Err(Error::new("X11", format!("Diaplay server :{} is not running.", self.display)))?
        }
            
        match var("XAUTHORITY") {
            Ok(env) => { 
                if env == "" {
                    Err(Error::new("X11", format!("XAUTHORITY is unset.")))? 
                }
                if ! Path::new(&env).exists() {
                    Err(Error::new("X11", format!("XAUTHORITY path is invalid.")))?
                }
            },
            Err(_) => Err(Error::new("X11", format!("XAUTHORITY is unset.")))? 
        }  
        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) {
        let xauth = var("XAUTHORITY").unwrap();
        let container_xauth = format!("/run/user/{}/Xauthority", nix::unistd::geteuid());  
        let display = format!(":{}", self.display);
        args.env("DISPLAY", display);
        args.robind(xauth, &container_xauth);
        args.env("XAUTHORITY", container_xauth);
    }
}

fn is_default(var: &i8) -> bool {
    var == &0
}

fn default_display() -> i8 {
    0
}
