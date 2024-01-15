/*
 * pacwrap-agent
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
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

use pacwrap_core::{
    err,
    utils::{arguments::Operand, Arguments},
    Error,
};

use crate::error::AgentError;

mod agent;
mod error;

fn main() {
    let arguments = &mut Arguments::new().populate();
    let param = arguments.next().unwrap_or_default();
    let result = match param {
        Operand::Value("transact") => agent::transact(),
        _ => err!(AgentError::DirectExecution),
    };

    if let Err(error) = result {
        error.handle();
    }
}
