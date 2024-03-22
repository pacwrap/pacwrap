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
    fmt::{Display, Formatter},
    fs::{remove_dir_all, remove_file},
    path::Path,
};

use pacwrap_core::{
    config::{cache, ContainerCache},
    constants::{ARROW_GREEN, BOLD, DATA_DIR, RESET},
    err,
    impl_error,
    log::{Level::Info, Logger},
    process,
    utils::{arguments::Operand, prompt::prompt_targets, Arguments},
    Error,
    ErrorGeneric,
    ErrorKind,
    ErrorTrait,
    Result,
};

#[derive(Debug)]
enum DeleteError {
    ContainerRunning(String),
}

impl_error!(DeleteError);

impl Display for DeleteError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::ContainerRunning(err) => write!(fmter, "Container '{}{}{}' has running processes.", *BOLD, err, *RESET),
        }?;

        write!(fmter, "\nTry 'pacwrap -h' for more information on valid operational parameters.")
    }
}

pub fn remove_containers(args: &mut Arguments) -> Result<()> {
    let mut targets = vec![];
    let mut no_confirm = false;
    let mut force = false;
    let mut logger = Logger::new("pacwrap-utils").init()?;

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('m') | Operand::Long("delete") => continue,
            Operand::ShortPos('t', val)
            | Operand::ShortPos('m', val)
            | Operand::ShortPos('r', val)
            | Operand::LongPos("target", val)
            | Operand::LongPos("delete", val)
            | Operand::LongPos("remove", val)
            | Operand::Value(val) => targets.push(val),
            Operand::Long("noconfirm") => no_confirm = true,
            Operand::Long("force") => force = true,
            _ => args.invalid_operand()?,
        }
    }

    let cache = cache::populate_config()?;
    let instances = cache.filter_target(&targets, vec![]);

    if instances.len() != targets.len() {
        for target in &targets {
            if !instances.contains(&target) {
                err!(ErrorKind::InstanceNotFound(target.to_string()))?;
            }
        }
    }

    if let (true, _) | (_, Ok(_)) = (no_confirm, prompt_targets(&instances, "Delete containers?", false)) {
        delete_roots(&cache, &mut logger, &instances, force)
    } else {
        Ok(())
    }
}

pub fn delete_roots(cache: &ContainerCache<'_>, logger: &mut Logger, targets: &Vec<&str>, force: bool) -> Result<()> {
    let process = process::list(&cache)?;
    let processes = process.filter_by_target(&targets);
    let containers = cache.filter_target_handle(&targets, vec![]);

    if processes.len() > 0 && !force {
        for process in processes {
            err!(DeleteError::ContainerRunning(process.instance().to_string()))?;
        }
    }

    for container in containers {
        let root = container.vars().root();
        let instance = container.vars().instance();
        let state = format!("{}/state/{instance}.dat", *DATA_DIR);

        remove_dir_all(root).prepend(|| format!("Failed to delete container root '{root}':"))?;

        if Path::new(&state).exists() {
            remove_file(&state).prepend_io(|| state)?;
        }

        eprintln!("{} Deleted container '{}{}{}' successfully.", *ARROW_GREEN, *BOLD, instance, *RESET);
        logger.log(Info, &format!("Deleted container {instance}"))?;
    }

    Ok(())
}
