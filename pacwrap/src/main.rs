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

use pacwrap_core::utils::arguments::{Arguments, Operand as Op};

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
        Op::Short('E') | Op::Long("exec") | Op::Value("shell") | Op::Value("run") => exec::execute(arguments),
        Op::Short('S') | Op::Long("sync") | Op::Value("sync") | Op::Value("init") => sync::synchronize(arguments),
        Op::Short('R') | Op::Long("remove") | Op::Value("remove") => remove::remove(arguments),
        Op::Short('Q') | Op::Long("query") | Op::Value("query") => query::query(arguments),
        Op::Short('P') | Op::Long("process") | Op::Value("ps") => proc::process(arguments),
        Op::Short('h') | Op::Long("help") | Op::Value("help") => manual::help(arguments),
        Op::Short('V') | Op::Long("version") | Op::Value("version")  => manual::print_version(arguments),
        Op::Short('U') | Op::Long("utils") => compat::execute_utils(arguments),
        Op::Long("compat") => compat::compat(arguments),
        _ => arguments.invalid_operand(),
    };

    if let Err(error) = result {
        error.handle();
    }
}
