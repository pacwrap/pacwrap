use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::*, permission::PermError::*, permission::Condition::*};
use crate::utils::check_socket;
use crate::constants::XDG_RUNTIME_DIR;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PULSEAUDIO {
    #[serde(skip_serializing_if = "is_default_socket", default = "default_socket")]
    socket: String,
}

#[typetag::serde]
impl Permission for PULSEAUDIO {
    fn check(&self) -> Result<Option<Condition>, PermError> {  
        if ! Path::new(&self.socket).exists() {
            Err(Warn(format!("Pulseaudio socket not found.")))?
        }

        if ! check_socket(&self.socket) {          
            Err(Warn(format!("'{}' is not a valid UNIX socket.", &self.socket)))?
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) {
        args.robind(&self.socket, default_socket());
    }

    fn module(&self) -> &str {
        "PULSEAUDIO"
    }
}

fn is_default_socket(var: &String) -> bool {
    let default: &String = &default_socket();
    default == var
}

fn default_socket() -> String {
    format!("{}/pulse/native", *XDG_RUNTIME_DIR)
}
