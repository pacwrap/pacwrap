/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use crate::constants::{BOLD, RESET};

pub mod config;
pub mod constants;
pub mod error;
pub mod exec;
pub mod lock;
pub mod log;
pub mod process;
pub mod sync;
pub mod utils;

pub use error::*;

#[derive(Debug)]
pub enum ErrorKind {
    EnvVarUnset(&'static str),
    ProcessInitFailure(&'static str, std::io::ErrorKind),
    ProcessWaitFailure(&'static str, std::io::ErrorKind),
    IOError(String, std::io::ErrorKind),
    Message(&'static str),
    Termios(nix::errno::Errno),
    InstanceNotFound(String),
    DependencyNotFound(String, String),
    LinkerUninitialized,
    ThreadPoolUninitialized,
    ElevatedPrivileges,
}

impl Display for ErrorKind {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DependencyNotFound(dep, ins) =>
                write!(fmter, "Instance '{}{}{}': Dependency {}{}{} not found.", *BOLD, ins, *RESET, *BOLD, dep, *RESET),
            Self::Message(err) => write!(fmter, "{}", err),
            Self::EnvVarUnset(var) => write!(fmter, "${}{var}{} is unset.", *BOLD, *RESET),
            Self::ProcessInitFailure(exec, err) => write!(fmter, "Unable to initialize '{exec}': {err}"),
            Self::ProcessWaitFailure(exec, err) => write!(fmter, "Unable to wait on '{exec}': {err}"),
            Self::InstanceNotFound(ins) => write!(fmter, "Container '{}{ins}{}' not found.", *BOLD, *RESET),
            Self::IOError(ins, error) => write!(fmter, "'{ins}': {error}"),
            Self::ThreadPoolUninitialized => write!(fmter, "Threadpool uninitialized"),
            Self::LinkerUninitialized => write!(fmter, "Filesystem synchronization structure is uninitialized."),
            Self::Termios(errno) => write!(fmter, "Failed to restore termios parameters: {errno}."),
            Self::ElevatedPrivileges => write!(fmter, "Execution with elevated privileges is not supported."),
        }?;

        if let Self::Message(_) = self {
            write!(fmter, "\nTry 'pacwrap -h' for more information on valid operational parameters.")?;
        }

        Ok(())
    }
}

impl ErrorTrait for ErrorKind {
    fn code(&self) -> i32 {
        match self {
            ErrorKind::IOError(..) => 2,
            _ => 1,
        }
    }
}
