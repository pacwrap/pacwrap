#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

use crate::{config::Dbus, exec::args::ExecutionArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XDG_PORTAL;

#[typetag::serde]
impl Dbus for XDG_PORTAL {
    fn register(&self, args: &mut ExecutionArgs) { 
        args.dbus("call", "org.freedesktop.portal.*=*");
        args.dbus("broadcast", "org.freedesktop.portal.*=@/org/freedesktop/portal/*");
    }
}
