#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Dbus};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XDG_PORTAL;

#[typetag::serde]
impl Dbus for XDG_PORTAL {
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars) { 
        args.dbus("call", "org.freedesktop.portal.*=*");
        args.dbus("broadcast", "org.freedesktop.portal.*=@/org/freedesktop/portal/*");
    }
}
