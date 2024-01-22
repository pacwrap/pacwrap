/*
 * pacwrap
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

use pacwrap_core::utils::arguments::{Arguments, Operand};

mod compat;
mod exec;
mod manual;
mod proc;
mod query;
mod remove;
mod sync;

fn main() {
    let arguments = &mut Arguments::new().populate();
    let result = match arguments.next().unwrap_or_default() {
        Operand::Short('E') | Operand::Long("exec") => exec::execute(arguments),
        Operand::Short('S') | Operand::Long("sync") => sync::synchronize(arguments),
        Operand::Short('R') | Operand::Long("remove") => remove::remove(arguments),
        Operand::Short('Q') | Operand::Long("query") => query::query(arguments),
        Operand::Short('P') | Operand::Long("proc") => proc::process(arguments),
        Operand::Short('h') | Operand::Long("help") => manual::help(arguments),
        Operand::Short('U') | Operand::Long("utils") => compat::execute_utils(arguments),
        Operand::Short('V') | Operand::Long("version") => manual::print_version(arguments),
        Operand::Long("compat") => compat::compat(arguments),
        _ => arguments.invalid_operand(),
    };

    if let Err(error) = result {
        error.handle();
    }
}
