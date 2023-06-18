use crate::exec::args::ExecutionArgs;
use crate::config::vars::InsVars;

pub mod none;
mod x11;
mod env;
mod pulseaudio;
mod pipewire;
mod gpu;
mod net;
mod dev;

pub struct Error {
    error: String,
    mod_name: String
}

impl Error {
    pub fn new(name: impl Into<String>, err: impl Into<String>) -> Self {
        Self { error: err.into(), mod_name: name.into() }
    }

    pub fn error(&self) -> &String { &self.error }
    pub fn module(&self) -> &String { &self.mod_name }
}

#[typetag::serde(tag = "permission")]
pub trait Permission {
    fn check(&self) -> Result<(),Error>;
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars);
}
