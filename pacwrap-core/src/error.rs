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

use std::{any::Any, process::exit, fmt::{Display, Debug}};

use crate::{utils::{print_error, print_warning}, sync::SyncError};

pub type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug)]
pub struct Error {
    kind: Box<dyn ErrorTrait>,
}

impl Error {
    pub fn new(err: Box<dyn ErrorTrait>) -> Self {
        Self {
            kind: err,
        }
    }

    pub fn handle(&self) {
        //Temporary until command and control pipeline is implemented for pacwrap-agent
        match self.downcast::<SyncError>() {
            Ok(error) => match error {
                SyncError::TransactionFailureAgent => (),
                _ => print_error(&self.kind), 
            } 
            Err(_) => print_error(&self.kind),
        }

        exit(self.kind.code());
    }

    pub fn error(&self) -> i32 {
        print_error(&self.kind); 
        self.kind.code()
    }

    pub fn warn(&self) {
        print_warning(&self.kind);
    }

    pub fn downcast<T: 'static>(&self) -> std::result::Result<&T, &Self> {
        match self.kind.as_any().downcast_ref::<T>() {
            Some(inner) => Ok(inner), None => Err(self),
        }
    }
}

impl<T> Downcast for T where T: ErrorTrait + 'static {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<&Box<dyn ErrorTrait>> for String {
    fn from(value: &Box<dyn ErrorTrait>) -> Self {
        value.into()
    }
}
