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
    any::Any,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    process::exit,
    result::Result as StdResult,
};

use crate::constants::{BOLD_RED, BOLD_YELLOW, RESET};

pub type Result<T> = StdResult<T, Error>;

#[macro_export]
macro_rules! err {
    ( $x:expr ) => {
        Err(Error::new(Box::new($x)))
    };
}

#[macro_export]
macro_rules! error {
    ( $x:expr ) => {
        Error::new(Box::new($x))
    };
}

#[macro_export]
macro_rules! impl_error {
    ( $x:ident ) => {
        impl ErrorTrait for $x {
            fn code(&self) -> i32 {
                1
            }
        }
    };
}

pub trait ErrorTrait: Debug + Display + Downcast {
    fn code(&self) -> i32;
}

pub trait Downcast {
    fn as_any(&self) -> &dyn Any;
}

pub trait ErrorGeneric<R, E> {
    fn prepend<F>(self, f: F) -> Result<R>
    where
        F: FnOnce() -> String;
    fn prepend_io<F>(self, f: F) -> Result<R>
    where
        F: FnOnce() -> String;
    fn generic(self) -> Result<R>;
}

#[derive(Debug)]
pub enum ErrorType<'a> {
    Error(&'a Error),
    Warn(&'a Error),
    Fatal(&'a Error),
}

#[derive(Debug)]
struct GenericError {
    prepend: String,
    error: String,
}

#[derive(Debug)]
pub struct Error {
    kind: Box<dyn ErrorTrait>,
}

impl Error {
    pub fn new(err: Box<dyn ErrorTrait>) -> Self {
        Self { kind: err }
    }

    pub fn fatal(&self) -> ! {
        eprintln!("{}", ErrorType::Fatal(self));
        exit(self.kind.code())
    }

    pub fn error(&self) -> ! {
        eprintln!("{}", ErrorType::Error(self));
        exit(self.kind.code())
    }

    pub fn warn(&self) {
        eprintln!("{}", ErrorType::Warn(self))
    }

    #[allow(clippy::borrowed_box)]
    pub fn kind(&self) -> &Box<dyn ErrorTrait> {
        &self.kind
    }

    pub fn downcast<T: 'static>(&self) -> StdResult<&T, &Self> {
        match self.kind.as_any().downcast_ref::<T>() {
            Some(inner) => Ok(inner),
            None => Err(self),
        }
    }
}

impl Display for ErrorType<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Fatal(e) => write!(f, "{}fatal:{} {}", *BOLD_RED, *RESET, e.kind),
            Self::Error(e) => write!(f, "{}error:{} {}", *BOLD_RED, *RESET, e.kind),
            Self::Warn(e) => write!(f, "{}warning:{} {}", *BOLD_YELLOW, *RESET, e.kind),
        }
    }
}

impl<R, E> ErrorGeneric<R, E> for StdResult<R, E>
where
    E: Display,
{
    fn prepend<F>(self, f: F) -> Result<R>
    where
        F: FnOnce() -> String, {
        match self {
            Ok(f) => Ok(f),
            Err(err) => err!(GenericError {
                prepend: f(),
                error: err.to_string(),
            }),
        }
    }

    fn prepend_io<F>(self, f: F) -> Result<R>
    where
        F: FnOnce() -> String, {
        match self {
            Ok(f) => Ok(f),
            Err(err) => err!(GenericError {
                prepend: format!("'{}'", f()),
                error: err.to_string(),
            }),
        }
    }

    fn generic(self) -> Result<R> {
        match self {
            Ok(f) => Ok(f),
            Err(err) => err!(GenericError {
                prepend: "An error has occurred".into(),
                error: err.to_string(),
            }),
        }
    }
}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.prepend, self.error)
    }
}

impl_error!(GenericError);

impl<T> Downcast for T
where
    T: ErrorTrait + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
