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

use std::collections::HashMap;

use pacwrap_core::{
    config::{cache, init::init},
    err,
    error::*,
    log::Logger,
    sync::transaction::{TransactionAggregator, TransactionFlags, TransactionType},
    utils::{
        arguments::{Arguments, InvalidArgument, Operand as Op},
        check_root,
    },
};

pub fn remove(mut args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;

    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let action = {
        let mut recursive = 0;
        let mut cascade = false;

        while let Some(arg) = args.next() {
            match arg {
                Op::Short('s') | Op::Long("recursive") => recursive += 1,
                Op::Short('c') | Op::Long("cascade") => cascade = true,
                _ => continue,
            }
        }

        TransactionType::Remove(recursive > 0, cascade, recursive > 1)
    };

    engage_aggregator(action, &mut args, &mut logger)
}

fn engage_aggregator<'a>(action_type: TransactionType, args: &'a mut Arguments, log: &'a mut Logger) -> Result<()> {
    let cache = cache::populate()?;
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str, Vec<&'a str>> = HashMap::new();
    let mut current_target = None;

    if let Op::Nothing = args.next().unwrap_or_default() {
        err!(InvalidArgument::OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
            Op::Long("remove")
            | Op::Long("cascade")
            | Op::Long("recursive")
            | Op::Short('R')
            | Op::Short('c')
            | Op::Short('s')
            | Op::Short('t') => continue,
            Op::Long("dbonly") => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Op::Long("noconfirm") => action_flags = action_flags | TransactionFlags::NO_CONFIRM,
            Op::Long("force-foreign") => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Op::Short('p') | Op::Long("preview") => action_flags = action_flags | TransactionFlags::PREVIEW,
            Op::Short('f') | Op::Long("filesystem") => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC,
            Op::ShortPos('t', target) | Op::LongPos("target", target) | Op::ShortPos(_, target) => {
                cache.get_instance(target)?;
                current_target = Some(target);
                targets.push(target);
            }
            Op::LongPos(_, package) | Op::Value(package) =>
                if let Some(target) = current_target {
                    match queue.get_mut(target) {
                        Some(vec) => vec.push(package),
                        None => {
                            queue.insert(target, vec![package]);
                        }
                    }
                },
            _ => args.invalid_operand()?,
        }
    }

    if let None = current_target {
        err!(InvalidArgument::TargetUnspecified)?
    }

    Ok(TransactionAggregator::new(&cache, queue, log, action_flags, action_type, current_target).aggregate()?)
}
