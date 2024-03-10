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

use std::{
    collections::{HashMap, HashSet},
    fs::create_dir,
    io::ErrorKind as IOErrorKind,
};

use indexmap::IndexMap;
use pacwrap_core::{
    config::{cache, init::init, ContainerCache, ContainerHandle, ContainerType},
    constants::{ARROW_GREEN, BAR_GREEN, BOLD, RESET},
    err,
    error::*,
    log::{Level::Info, Logger},
    sync::{
        instantiate_trust,
        schema,
        transaction::{TransactionAggregator, TransactionFlags, TransactionType},
    },
    utils::{
        arguments::{Arguments, InvalidArgument::*, Operand as Op},
        check_root,
    },
    ErrorKind,
};

pub fn synchronize(args: &mut Arguments) -> Result<()> {
    check_root()?;
    init()?;

    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let mut cache = cache::populate()?;
    let action = {
        let mut u = 0;
        let mut y = 0;

        if let Op::Value("init") = args[0] {
            u = 1;
            y = 1;
        }

        while let Some(arg) = args.next() {
            match arg {
                Op::Short('y') | Op::Long("refresh") => y += 1,
                Op::Short('u') | Op::Long("upgrade") => u += 1,
                _ => continue,
            }
        }

        TransactionType::Upgrade(u > 0, y > 0, y > 1)
    };
    let create = create(args);

    if create {
        if let TransactionType::Upgrade(upgrade, refresh, _) = action {
            if !refresh {
                err!(UnsuppliedOperand("--refresh", "Required for container creation."))?
            } else if !upgrade {
                err!(UnsuppliedOperand("--upgrade", "Required for container creation."))?
            }
        }

        instantiate_trust()?;
        instantiate(&mut logger, &mut cache, acquire_depends(args)?)?;
    }

    engage_aggregator(&cache, action, args, &mut logger, create)
}

fn acquire_depends<'a>(args: &mut Arguments<'a>) -> Result<IndexMap<&'a str, (ContainerType, Vec<&'a str>)>> {
    let mut deps: IndexMap<&'a str, (ContainerType, Vec<&'a str>)> = IndexMap::new();
    let mut current_target = "";
    let mut instype = None;

    while let Some(arg) = args.next() {
        match arg {
            Op::Short('b') | Op::Long("base") => instype = Some(ContainerType::Base),
            Op::Short('s') | Op::Long("slice") => instype = Some(ContainerType::Slice),
            Op::Short('a') | Op::Long("aggregate") => instype = Some(ContainerType::Aggregate),
            Op::ShortPos('d', dep) | Op::LongPos("dep", dep) => match instype {
                Some(instance) => {
                    if let ContainerType::Base = instance {
                        err!(ErrorKind::Message("Dependencies cannot be assigned to base containers."))?
                    }

                    match dep.contains(",") {
                        true =>
                            for dep in dep.split(",") {
                                match deps.get_mut(current_target) {
                                    Some(d) => d.1.push(dep),
                                    None => err!(TargetUnspecified)?,
                                }
                            },
                        false => match deps.get_mut(current_target) {
                            Some(d) => d.1.push(dep),
                            None => err!(TargetUnspecified)?,
                        },
                    }
                }
                None => err!(TargetUnspecified)?,
            },
            Op::Short('t') | Op::Long("target") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('t', target) | Op::LongPos("target", target) => match instype {
                        Some(instype) => {
                            current_target = target;
                            deps.insert(current_target, (instype, vec![]));
                        }
                        None => err!(ErrorKind::Message("Container type not specified."))?,
                    },
                    _ => args.invalid_operand()?,
                },
                None => err!(TargetUnspecified)?,
            },
            _ => continue,
        }
    }

    for dep in deps.iter() {
        if let ContainerType::Base = dep.1 .0 {
            continue;
        }

        if dep.1 .1.is_empty() {
            err!(ErrorKind::Message("Dependencies not specified."))?
        }
    }

    if current_target.len() == 0 {
        err!(TargetUnspecified)?
    }

    Ok(deps)
}

fn create(args: &mut Arguments) -> bool {
    if let Op::Value("init") = args[0] {
        return true;
    }

    for arg in args {
        if let Op::Short('c') | Op::Long("create") = arg {
            return true;
        }
    }

    return false;
}

fn instantiate<'a>(
    logger: &mut Logger,
    cache: &mut ContainerCache<'a>,
    targets: IndexMap<&'a str, (ContainerType, Vec<&'a str>)>,
) -> Result<()> {
    println!("{} {}Instantiating container{}...{}", *BAR_GREEN, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for target in targets {
        cache.add(target.0, target.1 .0, target.1 .1)?;
        instantiate_container(logger, cache.get_instance(target.0)?)?;
    }

    Ok(())
}

fn instantiate_container<'a>(logger: &mut Logger, handle: &'a ContainerHandle<'a>) -> Result<()> {
    let ins = handle.vars().instance();
    let instype = handle.metadata().container_type();

    create_dir(handle.vars().root()).prepend_io(|| handle.vars().root().into())?;

    if let ContainerType::Aggregate | ContainerType::Base = instype {
        if let Err(err) = create_dir(handle.vars().home()) {
            if err.kind() != IOErrorKind::AlreadyExists {
                err!(ErrorKind::IOError(handle.vars().root().into(), err.kind()))?
            }
        }
    }

    if let ContainerType::Base = instype {
        schema::extract(handle, &None)?;
    }

    handle.save()?;
    logger.log(Info, &format!("Instantiation of {ins} complete.")).unwrap();
    println!("{} Instantiation of {ins} complete.", *ARROW_GREEN);
    Ok(())
}

fn engage_aggregator<'a>(
    cache: &ContainerCache<'a>,
    action_type: TransactionType,
    args: &'a mut Arguments,
    log: &'a mut Logger,
    create: bool,
) -> Result<()> {
    let mut flags = if create {
        TransactionFlags::CREATE | TransactionFlags::FORCE_DATABASE
    } else {
        TransactionFlags::NONE
    };
    let mut targets = HashSet::new();
    let mut queue = HashMap::new();
    let mut current_target = "";
    let mut base = false;

    if let Op::Nothing = args.next().unwrap_or_default() {
        err!(OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
            Op::Short('a')
            | Op::Short('s')
            | Op::Short('d')
            | Op::Short('y')
            | Op::Short('u')
            | Op::Short('c')
            | Op::Long("aggregate")
            | Op::Long("slice")
            | Op::Long("dep")
            | Op::Long("refresh")
            | Op::Long("upgrade")
            | Op::Long("create")
            | Op::LongPos("dep", _) => continue,
            Op::Short('b') | Op::Long("base") => base = true,
            Op::Short('o') | Op::Long("target-only") => flags = flags | TransactionFlags::TARGET_ONLY,
            Op::Short('f') | Op::Long("filesystem") => flags = flags | TransactionFlags::FILESYSTEM_SYNC,
            Op::Short('p') | Op::Long("preview") => flags = flags | TransactionFlags::PREVIEW,
            Op::Long("dbonly") => flags = flags | TransactionFlags::DATABASE_ONLY,
            Op::Long("force-foreign") => flags = flags | TransactionFlags::FORCE_DATABASE,
            Op::Long("noconfirm") => flags = flags | TransactionFlags::NO_CONFIRM,
            Op::Short('t') | Op::Long("target") => match args.next() {
                Some(arg) => match arg {
                    Op::ShortPos('t', target) | Op::LongPos("target", target) => {
                        cache.get_instance(target)?;
                        current_target = target;
                        targets.insert(target);

                        if base {
                            queue.insert(current_target.into(), vec!["base"]);
                            base = false;
                        }
                    }
                    _ => args.invalid_operand()?,
                },
                None => err!(TargetUnspecified)?,
            },
            Op::LongPos(_, package) | Op::ShortPos(_, package) | Op::Value(package) =>
                if current_target != "" {
                    if let Some(vec) = queue.get_mut(current_target) {
                        vec.push(package.into());
                    } else {
                        queue.insert(current_target.into(), vec![package]);
                    }
                },
            _ => args.invalid_operand()?,
        }
    }

    let targets: Option<Vec<&str>> = match flags.intersects(TransactionFlags::TARGET_ONLY | TransactionFlags::CREATE) {
        true => {
            if current_target == "" && !flags.contains(TransactionFlags::FILESYSTEM_SYNC) {
                err!(TargetUnspecified)?
            }

            if flags.contains(TransactionFlags::CREATE) {
                for cache in cache.registered_handles().iter().filter(|a| a.is_creation()) {
                    targets.extend(cache.metadata().dependencies());
                }
            }

            match flags.contains(TransactionFlags::FILESYSTEM_SYNC) {
                false => Some(targets.into_iter().collect()),
                true => None,
            }
        }
        false => None,
    };

    Ok(TransactionAggregator::new(cache, log, action_type)
        .flag(flags)
        .queue(queue)
        .target(targets)
        .aggregate()?)
}
