#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XdgPortal;

#[typetag::serde(name = "xdg_portal")]
impl Dbus for XdgPortal {
    fn register(&self, args: &mut ExecutionArgs) {
        args.dbus("call", "org.freedesktop.portal.*=*");
        args.dbus("broadcast", "org.freedesktop.portal.*=@/org/freedesktop/portal/*");
    }
}
