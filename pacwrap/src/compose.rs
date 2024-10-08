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

use std::{collections::HashMap, path::Path};

use pacwrap_core::{
    config::{cache, compose_handle, init::init, ContainerCache, ContainerHandle, ContainerType::*},
    constants::{ARROW_GREEN, BAR_GREEN, BOLD, RESET},
    err,
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
        prompt::prompt_targets,
    },
    Error,
    ErrorGeneric,
    ErrorKind,
    ErrorType,
    Result,
};

use crate::utils::delete::delete_roots;

pub fn compose(args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;

    let lock = Lock::new().lock()?;
    let result = engage_aggregator(args, &lock);

    if let Err(error) = lock.unlock() {
        eprintln!("{}", ErrorType::Error(&error));
    }

    result
}

fn delete_containers<'a>(
    cache: &'a ContainerCache<'a>,
    lock: &'a Lock,
    logger: &mut Logger,
    delete: &[&str],
    flags: &TransactionFlags,
    force: bool,
) -> Result<()> {
    let message = format!("Deleting existing container{}?", if delete.len() > 1 { "s" } else { "" });

    if flags.contains(TransactionFlags::NO_CONFIRM) {
        println!("{} {}{}...{}", *BAR_GREEN, *BOLD, &message, *RESET);
        delete_roots(cache, lock, logger, delete, force)?;
    } else if prompt_targets(delete, &message, false)? {
        delete_roots(cache, lock, logger, delete, force)?;
    }

    Ok(())
}

fn compose_handles<'a>(
    cache: &ContainerCache<'a>,
    compose: HashMap<&'a str, Option<&'a str>>,
) -> Result<HashMap<&'a str, ContainerHandle<'a>>> {
    let mut composed = HashMap::new();

    for (instance, config) in compose {
        let handle = compose_handle(instance, config)?;
        let container_type = handle.metadata().container_type();
        let depends = handle.metadata().dependencies();

        if let Symbolic = container_type {
            if depends.is_empty() {
                err!(ErrorKind::Message("Symbolic containers require at least one dependency."))?;
            }
        } else if let Base = container_type {
            if !depends.is_empty() {
                err!(ErrorKind::Message("Dependencies cannot be assigned to base containers."))?;
            }
        }

        for target in depends {
            cache.get_instance(target)?;
        }

        composed.insert(instance, handle);
    }

    Ok(composed)
}

fn instantiate<'a>(
    composed: HashMap<&'a str, ContainerHandle<'a>>,
    mut cache: ContainerCache<'a>,
    lock: &'a Lock,
    logger: &mut Logger,
) -> Result<ContainerCache<'a>> {
    lock.assert()?;
    println!("{} {}Instantiating container{}...{}", *BAR_GREEN, *BOLD, if composed.len() > 1 { "s" } else { "" }, *RESET);

    for (instance, handle) in composed {
        instantiate_container(&handle)?;

        match cache.get_instance_option(instance) {
            Some(_) => cache.replace(instance, handle)?,
            None => cache.add_handle(instance, handle)?,
        }

        logger.log(Info, &format!("Instantiation of {instance} complete.")).unwrap();
        println!("{} Instantiation of {instance} complete.", *ARROW_GREEN);
    }

    Ok(cache)
}

fn acquire_targets<'a>(
    cache: &'a ContainerCache<'a>,
    targets: &mut Vec<&'a str>,
    queue: &mut HashMap<&'a str, Vec<&'a str>>,
) -> Result<()> {
    let registered = cache.registered_handles();

    for handle in registered.iter().filter(|a| a.is_creation()) {
        let mut depends = handle.metadata().dependencies();
        let instance = handle.vars().instance();

        depends.extend(
            registered
                .iter()
                .filter(|a| !targets.contains(&a.vars().instance()) && a.metadata().dependencies().contains(&instance))
                .map(|a| a.vars().instance()),
        );
        queue.insert(instance, handle.metadata().explicit_packages());
        targets.push(instance);
        targets.extend(depends);
    }

    Ok(())
}

fn engage_aggregator(args: &mut Arguments, lock: &Lock) -> Result<()> {
    let mut cache = match args.into_iter().find(|a| *a == Op::Long("from-config")) {
        Some(_) => cache::populate_config(),
        None => cache::populate(),
    }?;
    let mut flags = TransactionFlags::CREATE | TransactionFlags::FORCE_DATABASE;
    let mut logger = Logger::new("pacwrap-compose").init()?;
    let mut targets = Vec::new();
    let mut delete = Vec::new();
    let mut compose = HashMap::new();
    let mut queue = HashMap::new();
    let mut force = false;
    let mut reinitialize = false;
    let mut current_target = None;

    if args.len() <= 1 {
        err!(OperationUnspecified)?
    }

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Op::Long("from-config") => continue,
            Op::Long("debug") => flags |= TransactionFlags::DEBUG,
            Op::Long("noconfirm") => flags |= TransactionFlags::NO_CONFIRM,
            Op::Long("disable-sandbox") => flags |= TransactionFlags::NO_ALPM_SANDBOX,
            Op::Long("reinitialize-all") =>
                for instance in cache.registered() {
                    if let Some(handle) = cache.get_instance_option(instance) {
                        if Path::new(handle.vars().root()).exists() {
                            delete.push(instance);
                        }

                        compose.insert(instance, None);
                    }
                },
            Op::Short('l') | Op::Long("lazy-load") => flags |= TransactionFlags::LAZY_LOAD_DB,
            Op::Short('f') | Op::Long("force") => force = true,
            Op::Short('r') | Op::Long("reinitialize") => reinitialize = true,
            Op::Short('t') | Op::Long("target") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('t', t) | Op::LongPos("target", t) => current_target = Some(t),
                    _ => args.invalid_operand()?,
                },
                None => err!(TargetUnspecified)?,
            },
            Op::LongPos(_, config) | Op::ShortPos(_, config) | Op::Value(config) => {
                let target = match current_target {
                    Some(target) => target,
                    None => match config.char_indices().filter(|a| a.1 == '.').last() {
                        Some((index, ..)) => config.split_at(index).0,
                        None => config,
                    },
                };
                let config = if reinitialize {
                    let handle = cache.get_instance(target)?;

                    if Path::new(handle.vars().root()).exists() {
                        delete.push(target);
                    }

                    Path::new(target).try_exists().prepend_io(|| target.into())?;

                    current_target.map(|_| config)
                } else {
                    Some(config)
                };

                compose.insert(target, config);
                current_target = None;
                reinitialize = false;
            }
            _ => args.invalid_operand()?,
        }
    }

    if compose.is_empty() {
        err!(ErrorKind::Message("Composition targets not specified."))?
    }

    if !delete.is_empty() {
        delete_containers(&cache, lock, &mut logger, &delete, &flags, force)?;
    }

    if flags.contains(TransactionFlags::LAZY_LOAD_DB) {
        print_warning("Database lazy-loading triggered by `-l/--lazy-load`; this feature is experimental.");
        print_warning("In future, manual intervention may be required for missing dependencies.");
        print_warning("See `--help compose` or the pacwrap(1) man page for further information.");
    }

    cache = instantiate(compose_handles(&cache, compose)?, cache, lock, &mut logger)?;
    acquire_targets(&cache, &mut targets, &mut queue)?;
    instantiate_trust()?;
    TransactionAggregator::new(&cache, &mut logger, TransactionType::Upgrade(true, true, false))
        .assert_lock(lock)?
        .target(Some(targets))
        .flag(flags)
        .queue(queue)
        .progress()
        .aggregate()
}
