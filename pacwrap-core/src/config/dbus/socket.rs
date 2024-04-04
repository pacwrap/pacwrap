use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Socket {
    policy: String,
    address: Vec<String>,
}

#[typetag::serde(name = "socket")]
impl Dbus for Socket {
    fn register(&self, args: &mut ExecutionArgs) {
        match self.policy.to_lowercase().as_str() {
            p if p == "call" || p == "talk" || p == "see" || p == "own" || p == "broadcast" =>
                for sock in self.address.iter() {
                    args.dbus(p, sock);
                },
            _ => {}
        }
    }
}
