use std::fmt::{Formatter, Display};

use crate::constants::{RESET, BOLD};

pub mod args;
pub mod utils;

#[derive(Debug, Clone)]
pub enum ExecutionError {
    InvalidPathVar(&'static str, std::io::ErrorKind),
    ExecutableUnavailable(String),
    RuntimeArguments,
    UnabsolutePath(String),
    UnabsoluteExec(String),
    DirectoryNotExecutable(String),
    SocketTimeout(String),
}

impl Display for ExecutionError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::InvalidPathVar(dir, err) => write!(fmter, "Invalid {}PATH{} variable '{dir}': {err}", *BOLD, *RESET),
            Self::ExecutableUnavailable(exec) => write!(fmter, "'{}': Not available in container {}PATH{}.", exec, *BOLD, *RESET),
            Self::UnabsolutePath(path) => write!(fmter, "'{}': {}PATH{} variable must be absolute", path, *BOLD, *RESET),
            Self::UnabsoluteExec(path) => write!(fmter, "'{}': Executable path must be absolute.", path), 
            Self::DirectoryNotExecutable(path) => write!(fmter, "'{}': Directories are not executables.", path),
            Self::SocketTimeout(socket) => write!(fmter, "Socket '{socket}': timed out."),
            Self::RuntimeArguments => write!(fmter, "Invalid runtime arguments."), 
        }
    }
}
