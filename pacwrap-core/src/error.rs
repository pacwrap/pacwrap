use std::{any::Any, process::exit, fmt::{Display, Debug}};

use crate::utils::{print_error, print_warning};

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
        print_error(&self.kind); 
        exit(self.kind.code());
    }

    pub fn error(&self) {
        print_error(&self.kind); 
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
