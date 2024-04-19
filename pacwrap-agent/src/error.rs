/*
 * pacwrap-agent
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
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

use std::fmt::{Display, Formatter};

use pacwrap_core::{
    constants::{BOLD, RESET},
    ErrorTrait,
};

#[derive(Debug)]
pub enum AgentError {
    DeserializationError(String),
    InvalidVersion(u8, u8, u8, u8, u8, u8),
    InvalidMagic(u32, u32),
    IOError(&'static str, std::io::ErrorKind),
    DirectExecution,
}

impl Display for AgentError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::DirectExecution => write!(fmter, "Direct execution of this binary is unsupported."),
            Self::InvalidMagic(magic, comparator) => write!(fmter, "Magic mismatch {} != {}", magic, comparator),
            Self::InvalidVersion(a, b, c, d, e, f) => {
                write!(fmter, "Version mismatch {}.{}.{} != {}.{}.{}", a, b, c, d, e, f)
            }
            Self::DeserializationError(error) => write!(fmter, "Deserilization error: {}", error),
            Self::IOError(file, error) => write!(fmter, "'{}{}{}' {}", *BOLD, file, *RESET, error),
        }
    }
}

impl ErrorTrait for AgentError {
    fn code(&self) -> i32 {
        match self {
            Self::InvalidMagic(..) => 5,
            Self::InvalidVersion(..) => 4,
            Self::DeserializationError(..) => 3,
            Self::IOError(..) => 2,
            _ => 1,
        }
    }
}
