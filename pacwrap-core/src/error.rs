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
    fn prepend<F>(self, f: F) -> StdResult<R, Error>
    where
        F: FnOnce() -> String;
    fn prepend_io<F>(self, f: F) -> StdResult<R, Error>
    where
        F: FnOnce() -> String;
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

    pub fn handle(&self) {
        eprintln!("{}error:{} {}", *BOLD_RED, *RESET, self.kind);
        exit(self.kind.code());
    }

    pub fn error(&self) -> i32 {
        eprintln!("{}error:{} {}", *BOLD_RED, *RESET, self.kind);
        self.kind.code()
    }

    pub fn warn(&self) {
        eprintln!("{}warning:{} {}", *BOLD_YELLOW, *RESET, self.kind);
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

impl_error!(GenericError);

impl<R, E> ErrorGeneric<R, E> for StdResult<R, E>
where
    E: Display,
{
    fn prepend<F>(self, f: F) -> StdResult<R, Error>
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

    fn prepend_io<F>(self, f: F) -> StdResult<R, Error>
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
}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.prepend, self.error)
    }
}

impl<T> Downcast for T
where
    T: ErrorTrait + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
