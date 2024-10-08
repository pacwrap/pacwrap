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

use std::collections::HashMap;

use pacwrap_core::{
    config::{cache, init::init, ContainerType},
    err,
    error::*,
    lock::Lock,
    log::Logger,
    sync::transaction::{TransactionAggregator, TransactionFlags, TransactionType},
    utils::{
        arguments::{Arguments, InvalidArgument::*, Operand as Op},
        check_root,
    },
    ErrorKind,
};

use crate::utils::delete::remove_containers;

pub fn remove(args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;

    if args.len() > 1 && (args[0] == Op::Value("rm") || args[1] == Op::Short('m') || args[1] == Op::Long("delete")) {
        return remove_containers(args);
    }

    let mut logger = Logger::new("pacwrap-sync").init()?;
    let action = action(args);
    let lock = Lock::new().lock()?;
    let result = engage_aggregator(action, args, &mut logger, &lock);

    if let Err(error) = lock.unlock() {
        eprintln!("{}", ErrorType::Error(&error));
    }

    result
}

fn action(args: &mut Arguments) -> TransactionType {
    let mut recursive = 0;
    let mut cascade = false;

    for arg in args.by_ref() {
        match arg {
            Op::Short('s') | Op::Long("recursive") => recursive += 1,
            Op::Short('c') | Op::Long("cascade") => cascade = true,
            _ => continue,
        }
    }

    TransactionType::Remove(recursive > 0, cascade, recursive > 1)
}

fn engage_aggregator<'a>(
    action_type: TransactionType,
    args: &'a mut Arguments,
    log: &'a mut Logger,
    lock: &'a Lock,
) -> Result<()> {
    let cache = cache::populate()?;
    let mut flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str, Vec<&'a str>> = HashMap::new();
    let mut current_target = None;

    if let Op::Nothing = args.next().unwrap_or_default() {
        err!(OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
            Op::Long("remove")
            | Op::Long("cascade")
            | Op::Long("recursive")
            | Op::Short('R')
            | Op::Short('c')
            | Op::Short('s') => continue,
            Op::Long("debug") => flags |= TransactionFlags::DEBUG,
            Op::Long("dbonly") => flags |= TransactionFlags::DATABASE_ONLY,
            Op::Long("noconfirm") => flags |= TransactionFlags::NO_CONFIRM,
            Op::Long("force-foreign") => flags |= TransactionFlags::FORCE_DATABASE,
            Op::Long("disable-sandbox") => flags |= TransactionFlags::NO_ALPM_SANDBOX,
            Op::Short('p') | Op::Long("preview") => flags |= TransactionFlags::PREVIEW,
            Op::Short('f') | Op::Long("filesystem") => flags |= TransactionFlags::FILESYSTEM_SYNC,
            Op::Short('t') | Op::Long("target") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('t', target) | Op::LongPos("target", target) => {
                        if let ContainerType::Symbolic = cache.get_instance(target)?.metadata().container_type() {
                            err!(ErrorKind::Message("Symbolic containers cannot be transacted."))?;
                        }

                        current_target = Some(target);
                        targets.push(target);
                    }
                    _ => args.invalid_operand()?,
                },
                None => err!(TargetUnspecified)?,
            },
            Op::LongPos(_, package) | Op::ShortPos(_, package) | Op::Value(package) =>
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

    if current_target.is_none() {
        err!(TargetUnspecified)?
    }

    TransactionAggregator::new(&cache, log, action_type)
        .assert_lock(lock)?
        .target(Some(targets))
        .flag(flags)
        .queue(queue)
        .aggregate()
}
