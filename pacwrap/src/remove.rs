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

use pacwrap_core::{log::Logger,
    sync::transaction::TransactionType,
    utils::{arguments::Operand, check_root},
    utils::arguments::{Arguments, InvalidArgument},
    sync::transaction::{TransactionFlags, TransactionAggregator}, 
    config::{cache, init::init},
    error::*, 
    err};

pub fn remove(mut args: &mut Arguments) -> Result<()> {
    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let action = {
        let mut recursive = 0;
        let mut cascade = false;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('s') | Operand::Long("recursive") => recursive += 1,
                Operand::Short('c') | Operand::Long("cascade") => cascade = true,
                _ => continue,
            }
        }

        TransactionType::Remove(recursive > 0 , cascade, recursive > 1) 
    };

    check_root()?;
    init()?;
    engage_aggregator(action, &mut args, &mut logger)
}

fn engage_aggregator<'a>(
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    log: &'a mut Logger) -> Result<()> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str,Vec<&'a str>> = HashMap::new();
    let mut current_target = None;

    if let Operand::None = args.next().unwrap_or_default() { 
        err!(InvalidArgument::OperationUnspecified)?
    }

    while let Some(arg) = args.next() {
        match arg {
            Operand::Long("remove")
                | Operand::Long("cascade") 
                | Operand::Long("recursive") 
                | Operand::Short('R')
                | Operand::Short('c')  
                | Operand::Short('s') 
                | Operand::Short('t') => continue,  
            Operand::Long("noconfirm") 
                => action_flags = action_flags | TransactionFlags::NO_CONFIRM,                  
            Operand::Short('p') 
                | Operand::Long("preview") 
                => action_flags = action_flags | TransactionFlags::PREVIEW, 
            Operand::Long("db-only") 
                => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") 
                => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Short('f') 
                | Operand::Long("filesystem") 
                => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) 
                | Operand::ShortPos(_, target) => {
                current_target = Some(target);
                targets.push(target);
            },
            Operand::LongPos(_, package)
            | Operand::Value(package) => if let Some(target) = current_target {
                match queue.get_mut(target) {
                    Some(vec) => vec.push(package),
                    None => { queue.insert(target, vec!(package)); },
                }
            },
            _ => args.invalid_operand()?,
        }
    }
        
    if let None = current_target {
        err!(InvalidArgument::TargetUnspecified)?
    }

    Ok(TransactionAggregator::new(&cache::populate()?, 
        queue, 
        log, 
        action_flags, 
        action_type, 
        current_target)
        .aggregate()?)
}
