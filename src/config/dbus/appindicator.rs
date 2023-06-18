use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Dbus};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct APPINDICATOR;

#[typetag::serde]
impl Dbus for APPINDICATOR {
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) {
        args.dbus("broadcast", "org.kde.StatusNotifierWatcher=@/StatusNotifierWatcher");
    }
}
