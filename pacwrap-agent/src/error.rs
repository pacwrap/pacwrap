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

use thiserror::Error as ThisError;

use pacwrap_core::{
    ErrorTrait,
    constants::{BOLD, RESET},
};

#[derive(ThisError, Debug)]
pub enum AgentError {
    #[error("Deserilization error: {0}")]
    DeserializationError(String),
    #[error("Version mismatch {0}.{1}.{2} != {3}.{4}.{5}")]
    InvalidVersion(u8, u8, u8, u8, u8, u8),
    #[error("Magic mismatch {0} != {1}")]
    InvalidMagic(u32, u32),
    #[error("'{bold}{0}{reset}' {1}", bold=*BOLD, reset=*RESET)]
    IOError(&'static str, std::io::ErrorKind),
    #[error("Direct execution of this binary is unsupported.")]
    DirectExecution,
}

impl ErrorTrait for AgentError {
    fn code(&self) -> i32 {
        match self {
            Self::InvalidMagic(..) => 6,
            Self::InvalidVersion(..) => 5,
            Self::DeserializationError(..) => 4,
            Self::IOError(..) => 3,
            _ => 2,
        }
    }
}
