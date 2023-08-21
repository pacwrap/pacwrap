use crate::exec::args::ExecutionArgs;

mod socket;
mod appindicator;
mod xdg_portal;

#[typetag::serde(tag = "permission")]
pub trait Dbus {
    fn register(&self, args: &mut ExecutionArgs);
}
