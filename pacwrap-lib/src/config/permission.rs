use crate::exec::args::ExecutionArgs;

use dyn_clone::{DynClone, clone_trait_object};

pub mod none;
mod display;
mod pulseaudio;
mod pipewire;
mod env;
mod gpu;
mod net;
mod dev;

pub enum Condition {
    Success,
    SuccessWarn(String),
    Nothing
}

pub enum PermError {
    Fail(String),
    Warn(String),
}

#[typetag::serde(tag = "permission")]
pub trait Permission: DynClone {
    fn check(&self) -> Result<Option<Condition>, PermError>;
    fn register(&self, args: &mut ExecutionArgs);
    fn module(&self) -> &str;
}

clone_trait_object!(Permission);
