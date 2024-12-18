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
    env,
    fmt::{Display, Formatter},
    ops::Index,
};

use crate::{err, error::*, impl_error};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Operand<'a> {
    Short(char),
    ShortPos(char, &'a str),
    Long(&'a str),
    LongPos(&'a str, &'a str),
    Value(&'a str),
    ShortEmpty,
    Nothing,
}

#[derive(Debug)]
pub struct Arguments<'a> {
    inner: Vec<&'a str>,
    operands: Vec<Operand<'a>>,
    idx: usize,
    cur: usize,
}

#[derive(Debug, Clone)]
pub enum InvalidArgument {
    InvalidOperand(String),
    UnsuppliedOperand(&'static str, &'static str),
    OperationUnspecified,
    TargetUnspecified,
}

impl_error!(InvalidArgument);

impl Display for InvalidArgument {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::UnsuppliedOperand(params, message) => write!(fmter, "Option '{params}': {message}"),
            Self::InvalidOperand(oper) => write!(fmter, "Invalid option '{oper}'"),
            Self::OperationUnspecified => write!(fmter, "Operation not specified."),
            Self::TargetUnspecified => write!(fmter, "Target not specified."),
        }?;

        write!(fmter, "\nTry 'pacwrap -h' for more information on valid operational parameters.")
    }
}

impl Default for Arguments<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Arguments<'a> {
    pub fn new() -> Self {
        Self {
            inner: env::args().skip(1).map(|a| a.leak() as &'a str).collect::<Vec<_>>(),
            operands: Vec::new(),
            idx: 0,
            cur: 0,
        }
    }

    pub fn populate(mut self) -> Arguments<'a> {
        for string in &self.inner {
            match string {
                string if string.starts_with("--") =>
                    if string.contains('=') {
                        let value: Vec<&'a str> = string[2 ..].splitn(2, '=').collect();

                        self.operands.push(Operand::Long(value[0]));
                        self.operands.push(Operand::LongPos(value[0], value[1]));
                    } else {
                        self.operands.push(Operand::Long(&string[2 ..]));
                    },
                string if string.starts_with("-") =>
                    if string.len() > 1 {
                        for operand in string[1 ..].chars() {
                            self.operands.push(Operand::Short(operand));
                        }
                    } else {
                        self.operands.push(Operand::ShortEmpty);
                    },
                _ => self.operands.push(match self.operands.last() {
                    Some(last) => match last {
                        Operand::Short(c) => Operand::ShortPos(*c, string),
                        Operand::Long(s) => Operand::LongPos(s, string),
                        _ => Operand::Value(string),
                    },
                    None => Operand::Value(string),
                }),
            }
        }

        self
    }

    //#[deprecated]
    pub fn target(&mut self) -> Result<&'a str> {
        for op in self.into_iter() {
            if let Operand::ShortPos(_, name) | Operand::LongPos(_, name) | Operand::Value(name) = op {
                return Ok(name);
            }
        }

        err!(InvalidArgument::TargetUnspecified)
    }

    pub fn set_index(&mut self, index: usize) {
        self.idx = index;
        self.cur = index;
    }

    pub fn invalid_operand(&self) -> Result<()> {
        match self.operands.get(self.cur) {
            Some(oper) => err!(InvalidArgument::InvalidOperand(oper.to_string())),
            None => err!(InvalidArgument::OperationUnspecified),
        }
    }

    pub fn inner(&self) -> &[&'a str] {
        &self.inner
    }

    pub fn into_inner(&self, skip: usize) -> Vec<&'a str> {
        self.inner.iter().copied().skip(skip).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.operands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.operands.len()
    }
}

impl<'a> Index<usize> for Arguments<'a> {
    type Output = Operand<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.operands[index]
    }
}

impl<'a> Iterator for Arguments<'a> {
    type Item = Operand<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cur = self.idx;

        if self.cur < self.operands.len() {
            self.idx += 1;
            Some(self.operands[self.cur])
        } else {
            self.idx = 0;
            None
        }
    }
}

impl Display for Operand<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Long(str) => write!(fmt, "--{}", str),
            Operand::LongPos(str, eq) => write!(fmt, "--{}={}", str, eq),
            Operand::Short(char) => write!(fmt, "-{}", char),
            Operand::ShortPos(str, eq) => write!(fmt, "-{} {}", str, eq),
            Operand::Value(str) => write!(fmt, "{}", str),
            Operand::ShortEmpty => write!(fmt, "-"),
            Operand::Nothing => write!(fmt, "None"),
        }
    }
}

impl Default for &Operand<'_> {
    fn default() -> Self {
        &Operand::Nothing
    }
}

impl Default for Operand<'_> {
    fn default() -> Self {
        Self::Nothing
    }
}
