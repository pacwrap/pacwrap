/*
 * pacwrap
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

use alpm::PackageReason;

use pacwrap_core::{
    config,
    constants::{BOLD_GREEN, RESET},
    err,
    error::*,
    sync::{instantiate_alpm, transaction::TransactionFlags},
    utils::{
        arguments::{Arguments, InvalidArgument, Operand},
        check_root,
    },
};

use crate::help::{help, HelpTopic};

pub fn query(args: &mut Arguments) -> Result<()> {
    let mut flags: TransactionFlags = TransactionFlags::NONE;
    let mut target = "";
    let mut explicit = false;
    let mut quiet = false;

    check_root()?;

    while let Some(arg) = args.next() {
        match arg {
            Operand::Long("debug") => flags |= TransactionFlags::DEBUG,
            Operand::Long("target") | Operand::Short('t') => continue,
            Operand::Short('h') | Operand::Long("help") => return help(args, &HelpTopic::Query),
            Operand::Short('e') | Operand::Long("explicit") => explicit = true,
            Operand::Short('q') | Operand::Long("quiet") => quiet = true,
            Operand::LongPos(_, t) | Operand::ShortPos(_, t) | Operand::Value(t) => target = t,
            _ => args.invalid_operand()?,
        }
    }

    if target.is_empty() {
        err!(InvalidArgument::TargetUnspecified)?
    }

    let handle = config::provide_handle(target)?;
    let handle = instantiate_alpm(&handle, &flags)?;

    for pkg in handle.localdb().pkgs() {
        if explicit && pkg.reason() != PackageReason::Explicit {
            continue;
        }

        match quiet {
            true => println!("{} ", pkg.name()),
            false => println!("{} {}{}{} ", pkg.name(), *BOLD_GREEN, pkg.version(), *RESET),
        }
    }

    Ok(())
}
