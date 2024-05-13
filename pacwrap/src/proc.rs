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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

use indexmap::IndexMap;
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use pacwrap_core::{
    config::cache,
    constants::{ARROW_GREEN, BOLD, DIM, RESET},
    err,
    impl_error,
    process::{self, Process},
    utils::{
        arguments::{InvalidArgument, Operand},
        print_warning,
        prompt::prompt_targets,
        table::{ColumnAttribute, Table},
        Arguments,
    },
    Error,
    ErrorGeneric,
    ErrorTrait,
    Result,
};

#[derive(Debug)]
pub enum ProcError {
    NotEnumerable,
    SpecifiedNotEnumerable,
    InvalidSignalSpecified,
    InvalidDepthInput,
}

impl_error!(ProcError);

impl Display for ProcError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        match self {
            ProcError::NotEnumerable => write!(fmt, "No containers running for pacwrap to enumerate."),
            ProcError::SpecifiedNotEnumerable => write!(fmt, "Specified containers are not enumerable."),
            ProcError::InvalidSignalSpecified => write!(fmt, "Invalid UNIX signal specified."),
            ProcError::InvalidDepthInput => write!(fmt, "Depth can only be specified with a valid integer."),
        }?;

        write!(fmt, "\nTry 'pacwrap -h' for more information on valid operational parameters.")
    }
}

pub fn process(args: &mut Arguments) -> Result<()> {
    match args.next().unwrap_or_default() {
        Operand::Long("summary") | Operand::Short('s') => summary(args),
        Operand::Long("id-list") | Operand::Short('i') => process_id(args),
        Operand::Long("kill") | Operand::Short('k') => process_kill(args),
        Operand::Nothing =>
            if let Operand::Value("ps") = args[0] {
                summary(args)
            } else {
                err!(InvalidArgument::OperationUnspecified)
            },
        _ =>
            if let Operand::Value("ps") = args[0] {
                summary(args)
            } else {
                args.invalid_operand()
            },
    }
}

fn summary(args: &mut Arguments) -> Result<()> {
    let mut all = false;
    let mut max_depth = 1;
    let mut cmd = 0;
    let mut exec = 0;
    let mut instances = Vec::new();

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Value("ps") | Operand::Short('s') => continue,
            Operand::Short('d') | Operand::Short('t') | Operand::Long("depth") | Operand::Long("target") => continue,
            Operand::Short('x') | Operand::Long("exec") => exec += 1,
            Operand::Short('a') | Operand::Long("all") => all = true,
            Operand::Short('c') | Operand::Long("command") => cmd += 1,
            Operand::ShortPos('t', val) | Operand::LongPos("target", val) => instances.push(val),
            Operand::ShortPos('d', val) | Operand::LongPos("depth", val) => match val.parse() {
                Ok(val) => max_depth = val,
                Err(_) => err!(ProcError::InvalidDepthInput)?,
            },
            _ => args.invalid_operand()?,
        }
    }

    let col = (exec > 0, exec > 1 || cmd > 0, (exec > 0) as usize);
    let cache = cache::populate()?;
    let list = process::list(&cache)?;
    let list: Vec<_> = match instances.len() > 0 {
        true => list
            .list()
            .iter()
            .filter_map(|a| match instances.contains(&a.instance()) && (a.depth() <= max_depth || all) {
                true => Some(*a),
                false => None,
            })
            .collect(),
        false => list.list().iter().filter(|a| a.depth() <= max_depth || all).map(|a| *a).collect(),
    };

    if list.len() == 0 {
        err!(ProcError::NotEnumerable)?
    }

    let table_header = &match col {
        (true, false, _) => vec!["PID", "Container", "Executable"],
        (false, true, _) => vec!["PID", "Container", "Command"],
        (true, true, _) => vec!["PID", "Container", "Executable", "Arguments"],
        _ => vec!["PID", "Container"],
    };
    let mut table = if let (true, false, _) | (true, true, _) = col {
        Table::new()
            .header(&table_header)
            .col_attribute(0, ColumnAttribute::AlignRight)
            .col_attribute(1, ColumnAttribute::AlignLeftMax(15))
            .col_attribute(2, ColumnAttribute::AlignLeftMax(15))
    } else {
        Table::new()
            .header(&table_header)
            .col_attribute(0, ColumnAttribute::AlignRight)
            .col_attribute(1, ColumnAttribute::AlignLeftMax(15))
    };

    for process in list {
        let pid = process.pid().to_string();
        let ins = process.instance().to_string();
        let row = table.insert(match col {
            (true, false, _) => vec![pid, ins, process.exec().into()],
            (false, true, i) => vec![pid, ins, process.cmdlist_string(i)?],
            (true, true, i) => vec![pid, ins, process.exec().into(), process.cmdlist_string(i)?],
            _ => vec![pid, ins],
        });

        if process.fork() {
            fork_warn(process);
            table.mark(row);
        }
    }

    print!("{}{}", if table.marked() { "\n" } else { "" }, table.build().unwrap());
    Ok(())
}

fn process_id(args: &mut Arguments) -> Result<()> {
    let mut instance = Vec::new();
    let mut all = false;

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('d') => continue,
            Operand::Short('a') | Operand::Long("all") => all = true,
            Operand::Value(val)
            | Operand::ShortPos('i', val)
            | Operand::ShortPos('d', val)
            | Operand::LongPos("id-list", val) => instance.push(val),
            _ => args.invalid_operand()?,
        }
    }

    if instance.len() == 0 && !all {
        err!(InvalidArgument::TargetUnspecified)?
    }

    let cache = cache::populate()?;
    let list = process::list(&cache)?;
    let list: Vec<_> = match all {
        false => list.list().iter().filter(|a| instance.contains(&a.instance())).map(|a| *a).collect(),
        true => list.list(),
    };

    if list.len() == 0 {
        err!(ProcError::NotEnumerable)?
    }

    for idx in 0 .. list.len() {
        let process = list[idx];
        let pid = process.pid();

        if process.fork() {
            fork_warn(process);
        }

        if idx == list.len() - 1 {
            print!("{}", pid)
        } else {
            print!("{} ", pid);
        }
    }
    Ok(())
}

fn process_kill(args: &mut Arguments) -> Result<()> {
    let mut process: Vec<&str> = Vec::new();
    let mut sigint = Signal::SIGHUP;
    let mut all = false;
    let mut no_confirm = false;

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('s') | Operand::Long("signal") => continue,
            Operand::Long("noconfirm") => no_confirm = true,
            Operand::Long("all") => all = true,
            Operand::ShortPos('s', val) | Operand::LongPos("signal", val) =>
                sigint = match Signal::from_str(&val.to_uppercase()) {
                    Ok(sig) => sig,
                    Err(_) => err!(ProcError::InvalidSignalSpecified)?,
                },
            Operand::ShortPos(_, val) | Operand::LongPos(_, val) | Operand::Value(val) => process.push(val),
            _ => args.invalid_operand()?,
        }
    }

    if process.is_empty() && !all {
        err!(InvalidArgument::TargetUnspecified)?
    }

    let mut instances = IndexMap::new();
    let cache = cache::populate()?;
    let list = process::list(&cache)?;
    let list = match all {
        false => list
            .list()
            .iter()
            .filter_map(|a| match process.contains(&a.instance()) {
                true => Some(*a),
                false => None,
            })
            .collect(),
        true => list.list(),
    };

    if list.len() == 0 {
        err!(ProcError::SpecifiedNotEnumerable)?
    }

    for process in list.iter() {
        if process.fork() {
            fork_warn(process);
        }

        match instances.get(process.instance()) {
            Some(value) => instances.insert(process.instance(), value + 1),
            None => instances.insert(process.instance(), 1),
        };
    }

    let instances: Vec<String> = instances.iter().map(|a| format!("{} ({}{}{})", a.0, *DIM, a.1, *RESET)).collect();
    let instances = &instances.iter().map(|a| a.as_ref()).collect();

    match no_confirm || prompt_targets(instances, "Kill container processes?", false).is_ok() {
        true => kill_processes(&list, sigint),
        false => Ok(()),
    }
}

fn fork_warn(process: &Process) {
    print_warning(&format!(
        "Process fork detected with PID {}{}{} from an instance of {}{}{}.",
        *BOLD,
        process.pid(),
        *RESET,
        *BOLD,
        process.instance(),
        *RESET
    ));
}

fn kill_processes(process_list: &Vec<&Process>, sigint: Signal) -> Result<()> {
    for list in process_list {
        if let Err(err) = kill(Pid::from_raw(list.pid()), sigint).prepend(|| format!("Error killing '{}'", list.pid())) {
            err.warn();
            continue;
        }

        eprintln!("{} Killed process {} ({}{}{}) ", *ARROW_GREEN, list.pid(), *BOLD, list.instance(), *RESET);
    }

    Ok(())
}
