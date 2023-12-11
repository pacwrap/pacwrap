use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct APPINDICATOR;

#[typetag::serde]
impl Dbus for APPINDICATOR {
    fn register(&self, args: &mut ExecutionArgs) {
        args.dbus("broadcast", "org.kde.StatusNotifierWatcher=@/StatusNotifierWatcher");
    }
}
