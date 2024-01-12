use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::Dbus;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct APPINDICATOR;

#[typetag::serde]
impl Dbus for APPINDICATOR {
    fn register(&self, args: &mut ExecutionArgs) {
        args.dbus("broadcast", "org.kde.StatusNotifierWatcher=@/StatusNotifierWatcher");
    }
}
