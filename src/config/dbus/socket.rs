use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Dbus};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SOCKET {
    socket: String,
    address: Vec<String> 
}

#[typetag::serde]
impl Dbus for SOCKET {
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        match self.socket.to_lowercase().as_str() {
            p if p == "call" || p == "talk" || p == "see" || p == "own" || p == "broadcast" => {
                for sock in self.address.iter() {
                    args.dbus(p, sock);
                }
            },
            &_ => {}
        }
    }
}

