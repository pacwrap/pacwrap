use std::fmt::{Display, Formatter};

use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;

use dyn_clone::{DynClone, clone_trait_object};

pub mod home;
pub mod root;
mod to_home;
mod to_root;
mod dir;
mod sys;

pub enum Condition {
    Success,
    SuccessWarn(String),
    Nothing
}

#[derive(Debug, Clone)]
pub enum BindError {
    Fail(String),
    Warn(String),
}

impl Display for BindError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Fail(error) => write!(fmter, "{}", error),
            Self::Warn(error) => write!(fmter, "{}", error),
 
        }
    }
}

#[typetag::serde(tag = "mount")]
pub trait Filesystem: DynClone {
    fn check(&self, vars: &InsVars) -> Result<(), BindError>;
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars);
    fn module(&self) -> &'static str;
}

clone_trait_object!(Filesystem);

fn default_permission() -> String {
    "ro".into()
}

fn is_default_permission(var: &String) -> bool {
    var == "ro"
}
