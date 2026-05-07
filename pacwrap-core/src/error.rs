/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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

use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    path::Path,
    process::exit,
};

use thiserror::Error as ThisError;

use crate::{
    config::{BindError, ConfigError},
    constants::{ARROW_RED, BOLD, RESET},
    eprintln_error,
    eprintln_fatal,
    eprintln_warn,
    exec::{ExecutionError, path::PathError},
    lock::LockError,
    log::LoggerError,
    sync::{SyncError, filesystem::FilesystemSyncError},
    utils::{arguments::InvalidArgument, bytebuffer::BufferError, table::TableError},
};

pub type Result<T> = std::result::Result<T, Error<ErrorType>>;

#[macro_export]
macro_rules! impl_error {
    ( $x:ident ) => {
        impl ErrorTrait for $x {
            fn code(&self) -> i32 {
                1
            }
        }

        impl From<$x> for $crate::Error<$crate::ErrorType> {
            fn from(f: $x) -> Self {
                Self {
                    code: f.code(),
                    error: f.into(),
                }
            }
        }
    };
}

macro_rules! impl_from {
    ( $( $x:path => $y:literal),* ) => {
        $(
        impl From<$x> for $crate::Error<$crate::ErrorType> {
            fn from(f: $x) -> Self {
                Self {
                    code: $y,
                    error: f.into(),
                }
            }
        }
        )*
    };
}

#[derive(ThisError, Debug)]
pub struct Error<T> {
    pub error: T,
    pub code: i32,
}

pub trait ErrorExt {
    fn fatal(&self) -> !;
    fn error(&self) -> !;
    fn warn(&self);
}

pub trait ErrorTrait: Debug + Display {
    fn code(&self) -> i32;
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
#[allow(clippy::enum_variant_names)]
pub enum ErrorType {
    Config(#[from] ConfigError),
    Kind(#[from] ErrorKind),
    Bind(#[from] BindError),
    Path(#[from] PathError),
    Exec(#[from] ExecutionError),
    Sync(#[from] SyncError),
    FsSync(#[from] FilesystemSyncError),
    Args(#[from] InvalidArgument),
    Table(#[from] TableError),
    Buffer(#[from] BufferError),
    Lock(#[from] LockError),
    Log(#[from] LoggerError),
    Alpm(#[from] alpm::Error),
    TimeFormat(#[from] time::error::Parse),
    TimeParse(#[from] time::error::Format),
    AlpmRelease(#[from] alpm::ReleaseError),
    IoError(#[from] std::io::Error),
    Application(#[from] anyhow::Error),
}

impl_from! {
    anyhow::Error => 1,
    std::io::Error => 2,
    alpm::ReleaseError => 3,
    alpm::Error => 3
}

#[derive(ThisError, Debug)]
pub enum ErrorKind {
    EnvVarUnset(&'static str),
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
            Self::InstanceNotFound(ins) => write!(fmter, "Container '{}{ins}{}' not found.", *BOLD, *RESET),
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

impl_error!(ErrorKind);

impl<T> Display for Error<T>
where
    T: Debug + std::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl<T> From<T> for Error<T>
where
    T: ErrorTrait,
{
    fn from(f: T) -> Self {
        Self {
            code: f.code(),
            error: f,
        }
    }
}

impl ErrorExt for Error<ErrorType> {
    fn fatal(&self) -> ! {
        eprintln_fatal!("{}", self.error);
        exit(self.code)
    }

    fn error(&self) -> ! {
        eprintln_error!("{}", self.error);

        if let ErrorType::Sync(ref err) = self.error {
            match err {
                SyncError::TransactionFailure(_) => (),
                SyncError::SignalInterrupt => eprintln!("{} Transaction aborted.", *ARROW_RED),
                _ => eprintln!("{} Transaction failed.", *ARROW_RED),
            }
        }

        exit(self.code)
    }

    fn warn(&self) {
        eprintln_warn!("{}", self.error);
    }
}

impl<T> ErrorExt for T
where
    T: ErrorTrait + ?Sized,
{
    fn fatal(&self) -> ! {
        eprintln_fatal!("{self}");
        exit(self.code())
    }

    fn error(&self) -> ! {
        eprintln_error!("{self}");
        exit(self.code())
    }

    fn warn(&self) {
        eprintln_warn!("{self}");
    }
}

// Include path context with extension trait
pub trait PathContext {
    /// Since std library doesn't include path context, we have to add it ourselves.
    ///
    /// Usage:
    /// ```
    /// let path = "./test";
    ///
    /// File::open(&path).context_path(&path)?;
    /// ```
    ///
    /// Reference: https://github.com/rust-lang/rfcs/issues/2885
    ///            https://github.com/rust-lang/rfcs/issues/2885#issuecomment-2973324466
    fn context_path<T>(self, path: T) -> Self
    where
        T: AsRef<Path>;
}

impl PathContext for std::io::Error {
    fn context_path<T>(self, path: T) -> Self
    where
        T: AsRef<Path>, {
        Self::new(self.kind(), format!("\"{}\": {}", path.as_ref().display(), self))
    }
}

impl<T> PathContext for std::io::Result<T> {
    fn context_path<U>(self, path: U) -> Self
    where
        U: AsRef<Path>, {
        self.map_err(|error| error.context_path(path))
    }
}
