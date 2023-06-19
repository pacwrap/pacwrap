use crate::exec::args::ExecutionArgs;
use crate::config::InsVars;

pub mod home;
pub mod root;
mod to_home;
mod to_root;
mod dir;
mod sys;

pub struct Error {
    error: String,
    mod_name: String,
    critical: bool
}

impl Error {
    pub fn new(name: impl Into<String>, err: impl Into<String>, critical: bool) -> Self {
        Self { error: err.into(), mod_name: name.into(), critical: critical }
    }

    pub fn error(&self) -> &String { &self.error }
    pub fn module(&self) -> &String { &self.mod_name }
    pub fn critical(&self) -> &bool { &self.critical }
}

#[typetag::serde(tag = "mount")]
pub trait Filesystem {
    fn check(&self, vars: &InsVars) -> Result<(), Error>;
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars);
}

fn default_permission() -> String {
    "ro".into()
}

fn is_default_permission(var: &String) -> bool {
    if var == "ro" { return true; } false
}
