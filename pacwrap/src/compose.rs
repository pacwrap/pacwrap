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

use std::{collections::HashMap, fs::create_dir, path::Path};

use pacwrap_core::{
    config::{cache, compose_handle, init::init, ContainerCache, ContainerHandle, ContainerType},
    constants::{ARROW_GREEN, BAR_GREEN, BOLD, RESET},
    err,
    log::{Level::Info, Logger},
    sync::{
        schema,
        transaction::{TransactionAggregator, TransactionFlags, TransactionType},
    },
    utils::{
        arguments::{Arguments, InvalidArgument::*, Operand as Op},
        check_root,
        prompt::prompt_targets,
    },
    Error,
    ErrorGeneric,
    ErrorKind,
    Result,
};

use crate::utils::delete::delete_roots;

pub fn compose(args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;
    engage_aggregator(args)
}

fn instantiate_container<'a>(handle: &'a ContainerHandle<'a>) -> Result<()> {
    let instype = handle.metadata().container_type();
    let root = handle.vars().root();
    let home = handle.vars().home();

    create_dir(root).prepend_io(|| root.into())?;

    if let ContainerType::Aggregate | ContainerType::Base = instype {
        if !Path::new(home).exists() {
            create_dir(home).prepend_io(|| home.into())?;
        }
    }

    if let ContainerType::Base = instype {
        schema::extract(handle, &None)?;
    }

    handle.save()
}

fn delete_containers<'a>(
    cache: &'a ContainerCache<'a>,
    logger: &mut Logger,
    delete: &Vec<&str>,
    flags: &TransactionFlags,
    force: bool,
) -> Result<()> {
    let message = format!("Deleting existing container{}?", if delete.len() > 1 { "s" } else { "" });

    if flags.contains(TransactionFlags::NO_CONFIRM) {
        println!("{} {}{}...{}", *BAR_GREEN, *BOLD, message, *RESET);
        delete_roots(cache, logger, delete, force)?;
    } else if let Ok(_) = prompt_targets(&delete, message, false) {
        delete_roots(cache, logger, delete, force)?;
    }

    Ok(())
}

fn compose_containers<'a>(
    mut cache: ContainerCache<'a>,
    logger: &mut Logger,
    compose: HashMap<&'a str, Option<&'a str>>,
) -> Result<ContainerCache<'a>> {
    println!("{} {}Instantiating container{}...{}", *BAR_GREEN, *BOLD, if compose.len() > 1 { "s" } else { "" }, *RESET);

    for (instance, config) in compose {
        let handle = compose_handle(instance, config)?;

        for target in handle.metadata().dependencies() {
            cache.get_instance(target)?;
        }

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
    for handle in cache.registered_handles().iter().filter(|a| a.is_creation()) {
        let instance = handle.vars().instance();

        queue.insert(instance, handle.metadata().explicit_packages());
        targets.extend(handle.metadata().dependencies());
        targets.push(instance);
    }

    Ok(())
}

fn engage_aggregator<'a>(args: &mut Arguments) -> Result<()> {
    let mut cache = match args.into_iter().filter(|a| *a == Op::Long("from-config")).count() > 0 {
        true => cache::populate_config(),
        false => cache::populate(),
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
            Op::Short('t') | Op::Long("target") | Op::Long("from-config") => continue,
            Op::Long("noconfirm") => flags = flags | TransactionFlags::NO_CONFIRM,
            Op::Long("force") => force = true,
            Op::Long("reinitialize-all") =>
                for instance in cache.registered() {
                    if let Some(handle) = cache.get_instance_option(instance) {
                        if Path::new(handle.vars().root()).exists() {
                            delete.push(instance);
                        }

                        compose.insert(instance, None);
                    }
                },
            Op::Short('r') | Op::Long("reinitialize") => reinitialize = true,
            Op::ShortPos('t', target) | Op::LongPos("target", target) =>
                if !reinitialize {
                    current_target = Some(target);
                } else {
                    let handle = cache.get_instance(target)?;

                    if Path::new(handle.vars().root()).exists() {
                        delete.push(target);
                    }

                    compose.insert(target, None);
                    reinitialize = false;
                },
            Op::LongPos(_, target) | Op::ShortPos(_, target) | Op::Value(target) => {
                if !target.ends_with(".yml") {
                    err!(ErrorKind::Message("Unsupported file extension."))?;
                }

                if let Some(cur_target) = current_target {
                    compose.insert(cur_target, Some(target));
                    current_target = None;
                } else {
                    Path::new(target).try_exists().prepend_io(|| target.into())?;
                    compose.insert(target.split_at(target.len() - 4).0, Some(target));
                }
            }
            _ => args.invalid_operand()?,
        }
    }

    if compose.len() == 0 {
        err!(ErrorKind::Message("Composition targets not specified."))?
    }

    if delete.len() > 0 {
        delete_containers(&cache, &mut logger, &delete, &flags, force)?;
    }

    cache = compose_containers(cache, &mut logger, compose)?;
    acquire_targets(&cache, &mut targets, &mut queue)?;
    Ok(TransactionAggregator::new(&cache, &mut logger, TransactionType::Upgrade(true, true, false))
        .flag(flags)
        .target(Some(targets))
        .queue(queue)
        .aggregate()?)
}
