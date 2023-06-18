use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PULSEAUDIO {
    #[serde(skip_serializing_if = "is_default_socket", default = "default_socket")]
    socket: String,
}

#[typetag::serde]
impl Permission for PULSEAUDIO {
    fn check(&self) -> Result<(),Error> {  
        if ! Path::new(&self.socket).exists() {
            Err(Error::new("PULSEAUDIO", format!("Pulseaudio socket not present.")))?
        }
        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) {
        args.robind(&self.socket, default_socket());
    }
}

fn is_default_socket(var: &String) -> bool {
    let default: &String = &default_socket();
    default == var
}

fn default_socket() -> String {
    format!("/run/user/{}/pulse", nix::unistd::geteuid())
}