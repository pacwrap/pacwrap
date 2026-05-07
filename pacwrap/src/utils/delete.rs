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
    fmt::{Display, Formatter},
    fs::{Permissions, remove_dir_all, remove_file, set_permissions},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use anyhow::anyhow;
use pacwrap_core::{
    ErrorKind,
    PathContext,
    Result,
    config::{ContainerCache, cache},
    constants::{ARROW_GREEN, BOLD, DATA_DIR, RESET},
    eprintln_error,
    lock::Lock,
    log::{Level::Info, Logger},
    process,
    utils::{Arguments, arguments::Operand, prompt::prompt_targets},
};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
enum DeleteError {
    ContainerRunning(String),
    DeletionFailure(std::io::Error),
}

impl Display for DeleteError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::ContainerRunning(err) => write!(fmter, "Container '{}{}{}' has running processes.", *BOLD, err, *RESET),
            Self::DeletionFailure(err) => write!(fmter, "Failed to delete container: {err}"),
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

    let cache = cache::populate()?;
    let instances = cache.filter_target(&targets, vec![]);

    if instances.len() != targets.len() {
        for target in &targets {
            if !instances.contains(target) {
                Err(ErrorKind::InstanceNotFound(target.to_string()))?;
            }
        }
    }

    let lock = Lock::new().lock()?;

    if let (true, _) | (_, true) = (no_confirm, prompt_targets(&instances, "Delete containers?", false)?) {
        if let Err(err) = delete_roots(&cache, &lock, &mut logger, &instances, force) {
            eprintln_error!("{err}");
        }
    }

    lock.unlock()
}

pub fn delete_roots(cache: &ContainerCache<'_>, lock: &Lock, logger: &mut Logger, targets: &[&str], force: bool) -> Result<()> {
    let process = process::list(cache)?;
    let processes = process.filter_by_target(targets);
    let containers = cache.filter_target_handle(targets, vec![]);

    if !processes.is_empty() && !force {
        if let Some(process) = processes.first() {
            Err(anyhow!(DeleteError::ContainerRunning(process.instance().to_string())))?;
        }
    }

    for container in containers {
        let root = container.vars().root();
        let instance = container.vars().instance();
        let state = format!("{}/state/{instance}.dat", *DATA_DIR);

        lock.assert()?;

        for entry in WalkDir::new(root) {
            let Ok(path) = entry else {
                continue;
            };
            let path = path.path();
            let permissions = Permissions::from_mode(0o755);

            set_permissions(path, permissions).ok();
        }

        remove_dir_all(root)
            .context_path(root)
            .map_err(|e| anyhow!(DeleteError::DeletionFailure(e)))?;

        if Path::new(&state).exists() {
            remove_file(&state)
                .context_path(state)
                .map_err(|e| anyhow!(DeleteError::DeletionFailure(e)))?;
        }

        eprintln!("{} Deleted container '{}{}{}' successfully.", *ARROW_GREEN, *BOLD, instance, *RESET);
        logger.log(Info, &format!("Deleted container {instance}"))?;
    }

    Ok(())
}
