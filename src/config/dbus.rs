use crate::exec::args::ExecutionArgs;

use dyn_clone::{DynClone, clone_trait_object};

mod socket;
mod appindicator;
mod xdg_portal;

#[typetag::serde(tag = "permission")]
pub trait Dbus: DynClone {
    fn register(&self, args: &mut ExecutionArgs);
}

clone_trait_object!(Dbus);
