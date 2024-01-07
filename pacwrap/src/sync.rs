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

use std::{collections::HashMap, 
    fs::File, 
    io::Write};

use indexmap::IndexMap;
use pacwrap_core::{err,
    ErrorKind,
    error::*,
    log::Logger,
    sync::transaction::TransactionType,
    utils::arguments::{Arguments, 
        InvalidArgument, 
        Operand},
    config::{InstanceType, 
        InstanceHandle,
        init::init,
        cache},
    config::InstanceCache,
    sync::transaction::{TransactionFlags, TransactionAggregator}, 
    constants::{BAR_GREEN, BOLD, RESET, ARROW_GREEN}};

pub fn synchronize(args: &mut Arguments) -> Result<()> {
    let mut logger = Logger::new("pacwrap-sync").init().unwrap(); 
    let mut cache = cache::populate()?;
    let action = {
        let mut u = 0;
        let mut y = 0;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('y') | Operand::Long("refresh") => y += 1,
                Operand::Short('u') | Operand::Long("upgrade") => u += 1,
                _ => continue,
            }
        }

        TransactionType::Upgrade(u > 0, y > 0, y > 1)
    };

    init()?;

    if create(args) { 
        if let TransactionType::Upgrade(upgrade, refresh, _) = action { 
            if ! refresh {
                err!(InvalidArgument::UnsuppliedOperand("--refresh", "Required for container creation."))?
            } else if ! upgrade {
                err!(InvalidArgument::UnsuppliedOperand("--upgrade", "Required for container creation."))?
            }
        }

        instantiate(&mut logger, &mut cache, acquire_depends(args)?)?
    }

    engage_aggregator(&cache, action, args, &mut logger)
}

fn acquire_depends<'a>(args: &mut Arguments<'a>) -> Result<IndexMap<&'a str, (InstanceType, Vec<&'a str>)>> {
    let mut deps: IndexMap<&'a str, (InstanceType, Vec<&'a str>)> = IndexMap::new();
    let mut current_target = "";
    let mut instype = None;

    while let Some(arg) = args.next() { 
        match arg {
            Operand::ShortPos('d', dep) 
            | Operand::LongPos("dep", dep) => match deps.get_mut(current_target) {
                Some(d) => {
                    if let Some(instype) = instype {
                        if let InstanceType::BASE = instype {
                            err!(ErrorKind::Message("Dependencies cannot be assigned to base containers."))?
                        }
                    }
     
                    d.1.push(dep); 
                },
                None => err!(InvalidArgument::TargetUnspecified)?
            },
            Operand::Short('b') 
            | Operand::Long("base") => instype = Some(InstanceType::BASE),
            Operand::Short('s') 
            | Operand::Long("slice") => instype = Some(InstanceType::DEP),
            Operand::Short('r') 
            | Operand::Long("root") => instype = Some(InstanceType::ROOT),
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) => match instype {
                    Some(instype) => {
                        current_target = target;
                        deps.insert(current_target, (instype, vec!()));
                    },
                    None => err!(ErrorKind::Message("Container type not specified."))?,
            },          
            _ => continue,
        }
    }

    for dep in deps.iter() {
        if let InstanceType::BASE = dep.1.0 {
            continue;
        } 
            
        if dep.1.1.is_empty() {
        err!(ErrorKind::Message("Dependencies not specified."))?
        }
    }

    if current_target.len() == 0 {
        err!(InvalidArgument::TargetUnspecified)?
    }

    Ok(deps)
}


fn create(args: &mut Arguments) -> bool {
    for arg in args { 
        if let Operand::Short('c') | Operand::Long("create") = arg {
            return true;
        } 
    }

    return false;
}

fn instantiate<'a>(logger: &mut Logger, cache: &mut InstanceCache<'a>, targets: IndexMap<&'a str, (InstanceType, Vec<&'a str>)>) -> Result<()> { 
    println!("{} {}Instantiating container{}{}", *BAR_GREEN, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for target in targets {
        for dep in target.1.1.iter() {
            if let None = cache.get_instance(dep) {
                err!(ErrorKind::DependencyNotFound((*dep).into(), target.0.into()))?
            }
        }

        cache.add(target.0, target.1.0, target.1.1)?;

        match cache.get_instance(target.0) {
            Some(ins) => instantiate_container(logger, ins)?,
            None => err!(ErrorKind::InstanceNotFound(target.0.into()))?
        }
    }

    Ok(())
}

fn instantiate_container<'a>(logger: &mut Logger, handle: &'a InstanceHandle<'a>) -> Result<()> {
    let ins = handle.vars().instance();
    let instype = handle.metadata().container_type();

    if let Err(err) = std::fs::create_dir(handle.vars().root()) {
        err!(ErrorKind::IOError(handle.vars().root().into(), err.kind()))? 
    }

    if let InstanceType::ROOT | InstanceType::BASE = instype { 
        if let Err(err) = std::fs::create_dir(handle.vars().home()) {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                err!(ErrorKind::IOError(handle.vars().root().into(), err.kind()))?
            }
        }

        let bashrc = format!("{}/.bashrc", handle.vars().home());
        
        match File::create(&bashrc) {
            Ok(mut f) => if let Err(error) = write!(f, "PS1=\"$USER> \"") {
                err!(ErrorKind::IOError(bashrc, error.kind()))?
            },
            Err(error) => err!(ErrorKind::IOError(bashrc.clone(), error.kind()))?
        }; 
    }

    handle.save()?;
    logger.log(format!("Instantiation of {ins} complete.")).unwrap();
    println!("{} Instantiation of {ins} complete.", *ARROW_GREEN);
    Ok(())
}

fn engage_aggregator<'a>(
    cache: &InstanceCache<'a>,
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    log: &'a mut Logger) -> Result<()> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str ,Vec<&'a str>> = HashMap::new();
    let mut current_target = "";
    let mut base = false;

    if let Operand::None = args.next().unwrap_or_default() {
        err!(InvalidArgument::OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
                Operand::Short('d') 
                | Operand::Long("dep") | Operand::LongPos("dep", _)
                | Operand::Short('s') | Operand::Long("slice")
                | Operand::Short('r') | Operand::Long("root")
                | Operand::Short('t') | Operand::Long("target") 
                | Operand::Short('y') | Operand::Long("refresh")
                | Operand::Short('u') | Operand::Long("upgrade") => continue,
            Operand::Short('o') 
                | Operand::Long("target-only") 
                => action_flags = action_flags | TransactionFlags::TARGET_ONLY,
            Operand::Short('f') 
                | Operand::Long("filesystem") 
                => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::Short('p') 
                | Operand::Long("preview") 
                => action_flags = action_flags | TransactionFlags::PREVIEW,
            Operand::Short('c') 
                | Operand::Long("create") 
                => action_flags = action_flags | TransactionFlags::CREATE 
                    | TransactionFlags::FORCE_DATABASE,
            Operand::Short('b') | 
                Operand::Long("base") => base = true,
            Operand::Long("db-only") 
                => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") 
                => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Long("noconfirm") 
                => action_flags = action_flags | TransactionFlags::NO_CONFIRM, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) => { 
                if let None = cache.get_instance(target) {
                    err!(ErrorKind::InstanceNotFound(target.into()))?
                }

                current_target = target;
                targets.push(target);

                if base {         
                    queue.insert(current_target.into(), vec!("base", "pacwrap-base-dist")); 
                    base = false;  
                }
            },
            Operand::LongPos(_, package) 
            | Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { queue.insert(current_target.into(), vec!(package)); },
                }
            },
            _ => args.invalid_operand()?
        }
    }

    let current_target = match action_flags.intersects(TransactionFlags::TARGET_ONLY) {
        true => {
            if current_target == "" && ! action_flags.intersects(TransactionFlags::FILESYSTEM_SYNC) {
                err!(InvalidArgument::TargetUnspecified)?
            }

            Some(current_target)
        },
        false => {
            if let TransactionType::Upgrade(upgrade, refresh, _) = action_type {
                if ! upgrade && ! refresh {
                    err!(InvalidArgument::OperationUnspecified)?
                }
            }
       
            None
        }
    };

    Ok(TransactionAggregator::new(cache, 
        queue, 
        log, 
        action_flags, 
        action_type, 
        current_target).aggregate()?)
}
