use std::fmt::{Display, Formatter};

use std::env;

use crate::ErrorKind;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Operand<'a> {
    Short(char),
    ShortPos(char, &'a str),
    Long(&'a str),
    LongPos(&'a str, &'a str),
    Value(&'a str),
    None
}

#[derive(Debug)]
pub struct Arguments<'a> {
    values: Vec<&'a str>,
    operands: Vec<Operand<'a>>,
    idx: usize,
    cur: usize,
}

impl<'a> Arguments<'a> {
    pub fn new() -> Self {
        Self {
            values: env::args()
            .skip(1)
            .map(|a| { let a: &str = a.leak(); a })
            .collect::<Vec<_>>(),
            operands: Vec::new(),
            idx: 0,
            cur: 0,
        }
    }

    pub fn populate(mut self) -> Arguments<'a> { 
        for string in &self.values { 
            match string { 
                string if string.starts_with("--") => { 
                    if string.contains('=') {
                        let value: Vec<&'a str> = string[2..].splitn(2, '=').collect();

                        self.operands.extend([Operand::Long(value[0]), Operand::LongPos(value[0], value[1])]); 
                    } else {
                        self.operands.push(Operand::Long(&string[2..]));
                    }
                },
                string if string.starts_with("-") => if string.len() > 1 {
                    for operand in string[1..].chars() {
                        self.operands.push(Operand::Short(operand));
                    }
                },
                _ => self.operands.push(match self.operands.last() {
                    Some(last) => match last {
                        Operand::Short(c) => Operand::ShortPos(*c, string),
                        Operand::Long(s) => Operand::LongPos(*s, string),
                        _ => Operand::Value(string),
                    },
                    None => Operand::Value(string),
                }),
            }
        }

        self
    }

    pub fn target(&mut self) -> Result<&'a str, ErrorKind> {
        for op in self.into_iter() {
            if let Operand::ShortPos(_, name) 
            | Operand::LongPos(_, name) 
            | Operand::Value(name) = op {
                return Ok(name);
            }
        }

        Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))
    } 

    pub fn set_index(&mut self, index: usize) {
        self.idx = index;
        self.cur = index;
    }

    pub fn invalid_operand(&self) -> ErrorKind {
        match self.operands.get(self.cur) {
            Some(oper) => ErrorKind::Argument(InvalidArgument::InvalidOperand(oper.to_string().leak())),
            None => ErrorKind::Argument(InvalidArgument::OperationUnspecified),
        }
    }

    pub fn values(&self) -> &Vec<&'a str> {
        &self.values
    }
}
 
impl <'a>Iterator for Arguments<'a> {
    type Item = Operand<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cur = self.idx;

        if self.cur < self.operands.len() {
            self.idx += 1;
            Some(self.operands[self.cur])
        } else {        
            self.set_index(0);
            None
        }
    }
}

impl <'a>Default for &Operand<'a> {
    fn default() -> Self {
        &Operand::None
    }
}

impl <'a>Default for Operand<'a> {
    fn default() -> Self {
        Self::None
    }
}

impl <'a>Display for Operand<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)  -> std::fmt::Result {
        match self {
            Operand::Long(str) => write!(fmt, "--{}", str),
            Operand::LongPos(str, eq) => write!(fmt, "--{}={}", str, eq),
            Operand::Short(char) => write!(fmt, "-{}", char),
            Operand::ShortPos(str, eq) => write!(fmt, "-{} {}", str, eq),
            Operand::Value(str) => write!(fmt, "{}", str),
            Operand::None => write!(fmt, "None"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InvalidArgument {
    InvalidOperand(&'static str),
    UnsuppliedOperand(&'static str, &'static str),
    OperationUnspecified,
    TargetUnspecified,
}

impl Display for InvalidArgument {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       match self {
            Self::UnsuppliedOperand(params, message) => write!(fmter, "Option '{params}': {message}"),
            Self::InvalidOperand(oper) => write!(fmter, "Invalid option '{oper}'"), 
            Self::OperationUnspecified => write!(fmter, "Operation not specified."),
            Self::TargetUnspecified => write!(fmter, "Target not specified."),
        }
    }
}


