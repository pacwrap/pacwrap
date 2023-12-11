use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs,
    config::{Permission, permission::*},
    config::permission::{Condition::Success, PermError::Warn},
    constants::XDG_RUNTIME_DIR,
    utils::check_socket};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PIPEWIRE {
    #[serde(skip_serializing_if = "is_default_socket", default = "default_socket")]
    socket: String,
}

#[typetag::serde]
impl Permission for PIPEWIRE {
    fn check(&self) -> Result<Option<Condition>, PermError> {  
        if ! Path::new(&self.socket).exists() {
            Err(Warn(format!("Pipewire socket not found.")))?
        }

        if ! check_socket(&self.socket) {          
            Err(Warn(format!("'{}' is not a valid UNIX socket.", &self.socket)))?
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs) {
        args.robind(&self.socket, default_socket());
    }

    fn module(&self) -> &'static str {
        "PIPEWIRE"
    }
}

fn is_default_socket(var: &String) -> bool {
    let default: &String = &default_socket();
    default == var
}

fn default_socket() -> String {
    format!("{}/pipewire-0", *XDG_RUNTIME_DIR)
}
