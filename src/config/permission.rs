use crate::exec::args::ExecutionArgs;
use crate::config::vars::InsVars;

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
pub trait Permission {
    fn check(&self) -> Result<Option<Condition>, PermError>;
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars);
    fn module(&self) -> &str;
}
