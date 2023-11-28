use std::fmt::{Display, Formatter};

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

#[derive(Debug, Clone)]
pub enum PermError {
    Fail(String),
    Warn(String),
}

#[typetag::serde(tag = "permission")]
pub trait Permission: DynClone {
    fn check(&self) -> Result<Option<Condition>, PermError>;
    fn register(&self, args: &mut ExecutionArgs);
    fn module(&self) -> &'static str;
}

impl Display for PermError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Fail(error) => write!(fmter, "{}", error),
            Self::Warn(error) => write!(fmter, "{}", error),
 
        }
    }
}

clone_trait_object!(Permission);
