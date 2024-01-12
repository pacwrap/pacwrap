use std::fmt::Display;

use std::{env, process::exit};
use super::print_help_error;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Operand<'a> {
    Short(char),
    ShortPos(char, &'a str),
    Long(&'a str),
    LongPos(&'a str, &'a str),
    Value(&'a str),
    None
}

#[derive(Clone, Debug)]
pub struct Arguments<'a> {
    values: Vec<&'a str>,
    operands: Vec<Operand<'a>>,
    idx: usize,
    cur: usize,
}

impl<'a> Arguments<'a> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            operands: Vec::new(),
            idx: 0,
            cur: 0,
        }
    }

    pub fn parse(mut self) -> Arguments<'a> {
        for string in env::args().skip(1) {
            match string { 
                string if string.starts_with("--") => {
                    let string = string.leak();
                    
                    if string.contains('=') {
                        let value: Vec<&'a str> = string.split_at(2).1.split('=').collect();

                        self.operands.extend([Operand::Long(value[0]), Operand::LongPos(value[0], value[1])]); 
                    } else {
                        self.operands.push(Operand::Long(string.split_at(2).1));
                    }

                    self.values.push(string);
                },
                string if string.starts_with("-") => if string.len() > 1 {
                    let string = string.leak();

                    for operand in string.split_at(1).1.chars() {
                        self.operands.push(Operand::Short(operand));
                    }

                    self.values.push(string);
                },
                _ => {
                    let string = string.leak();
 
                    self.operands.push(match self.operands.last() {
                        Some(last) => match last {
                            Operand::Short(c) => Operand::ShortPos(*c, string),
                            Operand::Long(s) => Operand::LongPos(*s, string),
                            _ => Operand::Value(string),
                        },
                        None => Operand::Value(string),
                    });
                    self.values.push(string);
                }
            }
        }

        self
    }

    pub fn targets(&mut self) -> Vec<&'a str> {
        let mut targets = Vec::new();

        for op in self.into_iter() {
            if let Operand::ShortPos('t', name) | Operand::LongPos("target", name) = op { 
                targets.push(name); 
            }
        }
         
        targets
    }
    
    pub fn target(&mut self) -> &'a str {
        for op in self.into_iter() {
            if let Operand::ShortPos(_, name) 
            | Operand::LongPos(_, name) 
            | Operand::Value(name) = op {
                return name;
            }
        }

        print_help_error("Target not specified.");
        exit(1)
    }    

    pub fn set_index(&mut self, index: usize) {
        self.idx = index;
        self.cur = index;
    }

    pub fn invalid_operand(&self) -> String {
        match self.operands.get(self.cur) {
            Some(oper) => format!("Invalid option -- '{}'", oper),
            None => format!("Operation not specified."),
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
            Some(self.operands.as_slice()[self.cur])
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
            Operand::LongPos(str, eq) => write!(fmt, "--{} {}", str, eq),
            Operand::Short(char) => write!(fmt, "-{}", char),
            Operand::ShortPos(str, eq) => write!(fmt, "-{} {}", str, eq),
            Operand::Value(str) => write!(fmt, "{}", str),
            Operand::None => write!(fmt, "None"),
        }
    }
}
