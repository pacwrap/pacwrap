use std::process::exit;
use std::fmt::{Display, Formatter};

use config::ConfigError;
use exec::ExecutionError;
use utils::arguments::InvalidArgument;

use crate::{constants::{BOLD, RESET}, 
    utils::{print_error, print_warning}};

pub mod sync;
pub mod utils;
pub mod constants;
pub mod config;
pub mod log;
pub mod exec;

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Argument(InvalidArgument),
    Execution(ExecutionError),
    Config(ConfigError), 
    EnvVarUnset(&'static str),
    ProcessInitFailure(&'static str, std::io::ErrorKind),
    ProcessWaitFailure(&'static str, std::io::ErrorKind),
    IOError(String, std::io::ErrorKind), 
    Message(&'static str),
    InstanceNotFound(String), 
    DependencyNotFound(String, String), 
    LinkerUninitialized,
    ThreadPoolUninitialized,
}

impl ErrorKind {
    pub fn handle(&self) {
        print_error(self);
        eprintln!("Try 'pacwrap -h' for more information on valid operational parameters.");
        exit(self.into());
    }

    pub fn warn(&self) {
        print_warning(self);
    }
}

impl Display for ErrorKind {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::Argument(err) => write!(fmter, "{}", err),
            Self::Execution(err) => write!(fmter, "{}", err),
            Self::Config(err) => write!(fmter, "{}", err), 
            Self::Message(err) => write!(fmter, "{}", err), 
            Self::EnvVarUnset(var) => write!(fmter, "${}{var}{} is unset.", *BOLD, *RESET),
            Self::ProcessInitFailure(exec, err) => write!(fmter, "Unable to initialize '{exec}': {err}"), 
            Self::ProcessWaitFailure(exec, err) => write!(fmter, "Unable to wait on '{exec}': {err}"), 
            Self::InstanceNotFound(ins) => write!(fmter, "Instance {}{ins}{} not found.", *BOLD, *RESET),
            Self::DependencyNotFound(dep,ins) => write!(fmter, "Dependency {}{dep}{} not found for {}{ins}{}.", *BOLD, *RESET, *BOLD, *RESET),
            Self::IOError(ins, error) => write!(fmter, "'{ins}': {error}"),  
            Self::ThreadPoolUninitialized => write!(fmter, "Threadpool uninitialized"),
            Self::LinkerUninitialized => write!(fmter, "Filesystem synchronization structure is uninitialized."), 
        }
    }
}

impl From<&ErrorKind> for i32 {
    fn from(value: &ErrorKind) -> i32 {
        match value {
            ErrorKind::IOError(_,_) => 2, _ => 1, 
        }
    }
}

impl From<&ErrorKind> for String {
    fn from(value: &ErrorKind) -> Self {
        value.into()
    }
}

impl From<ErrorKind> for String {
    fn from(value: ErrorKind) -> Self {
        value.into()
    }
}
