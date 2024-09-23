use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppIndicator;

#[typetag::serde(name = "appindicator")]
impl Dbus for AppIndicator {
    fn register(&self, args: &mut ExecutionArgs) {
        args.dbus("broadcast", "org.kde.StatusNotifierWatcher=@/StatusNotifierWatcher");
        args.dbus("talk", "org.kde.StatusNotifierWatcher");
    }
}
