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

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use pacwrap_core::{
    config::{cache, init::init, ConfigError::AlreadyExists, ContainerCache, ContainerType},
    constants::{ARROW_GREEN, BAR_GREEN, BOLD, RESET},
    err,
    error::*,
    lock::Lock,
    log::{Level::Info, Logger},
    sync::{
        instantiate_container,
        instantiate_trust,
        transaction::{TransactionAggregator, TransactionFlags, TransactionType},
    },
    utils::{
        arguments::{Arguments, InvalidArgument::*, Operand as Op},
        check_root,
        print_warning,
    },
    ErrorKind,
};

pub fn synchronize(args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;

    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let mut cache = cache::populate()?;
    let (action, create) = action(args);
    let lock = Lock::new().lock()?;
    let result = engage_aggregator(&mut cache, &mut logger, args, &lock, action, create);

    if let Err(error) = lock.unlock() {
        error.error();
    }

    result
}

fn action(args: &mut Arguments) -> (TransactionType, bool) {
    let (mut y, mut u, mut i) = (0, 0, false);

    if let Op::Value("init") = args[0] {
        (y, u, i) = (1, 1, true);
    }

    while let Some(arg) = args.next() {
        match arg {
            Op::Short('y') | Op::Long("refresh") => y += 1,
            Op::Short('u') | Op::Long("upgrade") => u += 1,
            _ => continue,
        }
    }

    (TransactionType::Upgrade(u > 0, y > 0, y > 1), i)
}

fn instantiate<'a>(
    cache: &mut ContainerCache<'a>,
    lock: &'a Lock,
    logger: &mut Logger,
    action_type: &TransactionType,
    targets: IndexMap<&'a str, (ContainerType, Vec<&'a str>)>,
) -> Result<()> {
    if targets.is_empty() {
        err!(OperationUnspecified)?;
    }

    if let TransactionType::Upgrade(upgrade, refresh, _) = action_type {
        if !refresh {
            err!(UnsuppliedOperand("--refresh", "Required for container creation."))?
        } else if !upgrade {
            err!(UnsuppliedOperand("--upgrade", "Required for container creation."))?
        }
    }

    for (container, (container_type, deps)) in targets.iter() {
        if let (ContainerType::Base, true) = (container_type, deps.len() > 0) {
            err!(ErrorKind::Message("Dependencies cannot be assigned to base containers."))?
        } else if let (ContainerType::Aggregate | ContainerType::Slice, true) = (container_type, deps.is_empty()) {
            err!(ErrorKind::Message("Dependencies not specified."))?
        } else if let Some(_) = cache.get_instance_option(container) {
            err!(AlreadyExists(container.to_string()))?;
        }
    }

    lock.assert()?;
    println!("{} {}Instantiating container{}...{}", *BAR_GREEN, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for (container, (container_type, deps)) in targets {
        cache.add(container, container_type, deps)?;
        instantiate_container(cache.get_instance(container)?)?;
        logger.log(Info, &format!("Instantiation of {container} complete."))?;
        println!("{} Instantiation of {container} complete.", *ARROW_GREEN);
    }

    Ok(())
}

fn acquire_targets<'a>(
    cache: &'a ContainerCache<'a>,
    flags: &TransactionFlags,
    mut targets: HashSet<&'a str>,
) -> Result<Option<Vec<&'a str>>> {
    Ok(if flags.intersects(TransactionFlags::TARGET_ONLY | TransactionFlags::CREATE) {
        if flags.contains(TransactionFlags::CREATE) {
            for cache in cache.registered_handles().iter().filter(|a| a.is_creation()) {
                targets.extend(cache.metadata().dependencies());
            }
        }

        match flags.contains(TransactionFlags::FILESYSTEM_SYNC) {
            false => {
                if targets.is_empty() {
                    err!(TargetUnspecified)?;
                }

                Some(targets.into_iter().collect())
            }
            true => None,
        }
    } else {
        None
    })
}

fn engage_aggregator<'a>(
    mut cache: &mut ContainerCache<'a>,
    log: &'a mut Logger,
    args: &'a mut Arguments,
    lock: &'a Lock,
    action_type: TransactionType,
    init: bool,
) -> Result<()> {
    let mut flags = TransactionFlags::NONE;
    let mut create_targets: IndexMap<&'a str, (ContainerType, Vec<&'a str>)> = IndexMap::new();
    let mut targets = HashSet::new();
    let mut queue = HashMap::new();
    let mut current_target = None;
    let mut container_type = None;
    let mut create = init;

    if let Op::Nothing = args.next().unwrap_or_default() {
        err!(OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
            Op::Short('y') | Op::Short('u') | Op::Long("refresh") | Op::Long("upgrade") => continue,
            Op::Long("dbonly") => flags = flags | TransactionFlags::DATABASE_ONLY,
            Op::Long("force-foreign") => flags = flags | TransactionFlags::FORCE_DATABASE,
            Op::Long("noconfirm") => flags = flags | TransactionFlags::NO_CONFIRM,
            Op::Short('l') | Op::Long("lazy-load") => flags = flags | TransactionFlags::LAZY_LOAD_DB,
            Op::Short('o') | Op::Long("target-only") => flags = flags | TransactionFlags::TARGET_ONLY,
            Op::Short('f') | Op::Long("filesystem") => flags = flags | TransactionFlags::FILESYSTEM_SYNC,
            Op::Short('p') | Op::Long("preview") => flags = flags | TransactionFlags::PREVIEW,
            Op::Short('b') | Op::Long("base") => container_type = Some(ContainerType::Base),
            Op::Short('s') | Op::Long("slice") => container_type = Some(ContainerType::Slice),
            Op::Short('a') | Op::Long("aggregate") => container_type = Some(ContainerType::Aggregate),
            Op::Short('c') | Op::Long("create") => {
                container_type = None;
                create = true;
            }
            Op::Short('d') | Op::Long("dep") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('d', dep) | Op::LongPos("dep", dep) => match container_type {
                        Some(_) => {
                            let current_target = match current_target {
                                Some(target) => target,
                                None => err!(TargetUnspecified)?,
                            };
                            let (_, deps) = create_targets.get_mut(current_target).unwrap();

                            if dep.contains(",") {
                                for dep in dep.split(",").filter(|a| !a.is_empty()) {
                                    deps.push(dep);
                                }
                            } else {
                                deps.push(dep);
                            }
                        }
                        None => err!(ErrorKind::Message("Container type not specified."))?,
                    },
                    _ => args.invalid_operand()?,
                },
                None => err!(ErrorKind::Message("Dependencies not specified."))?,
            },
            Op::Short('t') | Op::Long("target") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('t', target) | Op::LongPos("target", target) => {
                        current_target = Some(target);
                        targets.insert(target);

                        if let (true, Some(container_type)) = (create, container_type) {
                            if let ContainerType::Base = container_type {
                                queue.insert(target.into(), vec!["base"]);
                            }

                            create_targets.insert(target, (container_type, vec![]));
                            create = init;
                        } else if let (true, None) = (create, container_type) {
                            err!(ErrorKind::Message("Container type not specified."))?;
                        } else if let ContainerType::Symbolic = cache.get_instance(target)?.metadata().container_type() {
                            err!(ErrorKind::Message("Symbolic containers cannot be transacted."))?;
                        }
                    }
                    _ => args.invalid_operand()?,
                },
                None => err!(TargetUnspecified)?,
            },
            Op::LongPos(_, package) | Op::ShortPos(_, package) | Op::Value(package) =>
                if let Some(current_target) = current_target {
                    if let Some(vec) = queue.get_mut(current_target) {
                        vec.push(package.into());
                    } else {
                        queue.insert(current_target, vec![package]);
                    }
                },
            _ => args.invalid_operand()?,
        }
    }

    if flags.contains(TransactionFlags::LAZY_LOAD_DB) {
        print_warning("Database lazy-loading triggered by `-l/--lazy-load`; this feature is experimental.");
        print_warning("In future, manual intervention may be required for missing dependencies.");
        print_warning("See `--help sync` or the pacwrap(1) man page for further information.");
    }

    if create_targets.len() > 0 || init {
        if flags.intersects(TransactionFlags::PREVIEW) {
            err!(ErrorKind::Message("Container creation cannot be previewed."))?;
        }

        flags = flags | TransactionFlags::CREATE | TransactionFlags::FORCE_DATABASE;
        instantiate_trust()?;
        instantiate(&mut cache, lock, log, &action_type, create_targets)?;
    }

    Ok(TransactionAggregator::new(cache, log, action_type)
        .assert_lock(lock)?
        .target(acquire_targets(cache, &flags, targets)?)
        .queue(queue)
        .flag(flags)
        .progress()
        .aggregate()?)
}
