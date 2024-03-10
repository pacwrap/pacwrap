/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
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
    fs,
    path::Path,
    process::exit,
    thread,
};

use signal_hook::{consts::*, iterator::Signals};

use crate::{
    config::{cache::ContainerCache, ContainerHandle, ContainerType},
    constants::{ARROW_GREEN, ARROW_RED, DATA_DIR, UNIX_TIMESTAMP},
    err,
    error::*,
    exec::{fakeroot_container, ExecutionType::NonInteractive},
    log::Logger,
    sync::{
        self,
        filesystem::{validate_fs_states, FileSystemStateSync},
        transaction::{Transaction, TransactionFlags, TransactionHandle, TransactionMetadata, TransactionState, TransactionType},
        SyncError,
    },
    utils::arguments::InvalidArgument,
    ErrorKind,
};

pub struct TransactionAggregator<'a> {
    queried: HashSet<&'a str>,
    updated: HashSet<&'a str>,
    pkg_queue: HashMap<&'a str, Vec<&'a str>>,
    action: TransactionType,
    filesystem_state: Option<FileSystemStateSync<'a>>,
    cache: &'a ContainerCache<'a>,
    keyring: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags,
    targets: Option<Vec<&'a str>>,
}

impl<'a> TransactionAggregator<'a> {
    pub fn new(inscache: &'a ContainerCache, log: &'a mut Logger, action_type: TransactionType) -> Self {
        Self {
            queried: HashSet::new(),
            updated: HashSet::new(),
            pkg_queue: HashMap::new(),
            filesystem_state: Some(FileSystemStateSync::new(inscache)),
            action: action_type,
            cache: inscache,
            keyring: false,
            logger: log,
            flags: TransactionFlags::NONE,
            targets: None,
        }
    }

    pub fn flag(mut self, flags: TransactionFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn queue(mut self, queue: HashMap<&'a str, Vec<&'a str>>) -> Self {
        self.pkg_queue = queue;
        self
    }

    pub fn target(mut self, targets: Option<Vec<&'a str>>) -> Self {
        self.targets = targets;
        self
    }

    pub fn aggregate(mut self) -> Result<()> {
        signal_trap(self.cache);

        let _timestamp = *UNIX_TIMESTAMP;
        let upgrade = match self.action {
            TransactionType::Upgrade(upgrade, refresh, force) => {
                if !upgrade && !refresh && !self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC) {
                    err!(InvalidArgument::OperationUnspecified)?
                }

                if refresh {
                    sync::synchronize_database(self.cache, force)?;
                }

                upgrade
            }
            TransactionType::Remove(..) => self.targets.is_some(),
        };
        let upstream = match self.targets.as_ref() {
            Some(targets) => self.cache.filter_target(targets, vec![ContainerType::Base, ContainerType::Slice]),
            None => self.cache.filter(vec![ContainerType::Base, ContainerType::Slice]),
        };
        let downstream = match self.targets.as_ref() {
            Some(targets) => self.cache.filter_target(targets, vec![ContainerType::Aggregate]),
            None => self.cache.filter(vec![ContainerType::Aggregate]),
        };
        let are_downstream = self.cache.count(vec![ContainerType::Aggregate]) > 0;

        if !validate_fs_states(&upstream) && are_downstream {
            let linker = self.fs_sync().unwrap();

            linker.refresh_state();
            linker.prepare(upstream.len());
            linker.engage(&upstream)?;
            linker.finish();
        }

        if upgrade {
            self.transaction(&upstream)?;
        }

        if self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC | TransactionFlags::CREATE) || self.updated.len() > 0 {
            if are_downstream {
                let file = self.cache.registered();
                let linker = self.fs_sync().unwrap();

                linker.filesystem_state();
                linker.prepare(file.len());
                linker.engage(&file)?;
                linker.finish();
            }

            self.filesystem_state = self.filesystem_state.unwrap().release();
        }

        if upgrade {
            self.transaction(&downstream)?;
        }

        println!("{} Transaction complete.", *ARROW_GREEN);
        Ok(())
    }

    pub fn transaction(&mut self, containers: &Vec<&'a str>) -> Result<()> {
        for ins in containers.iter() {
            if self.queried.contains(ins) {
                continue;
            }

            let inshandle = match self.cache.get_instance_option(ins) {
                Some(ins) => ins,
                None => continue,
            };

            self.queried.insert(ins);
            self.transaction(
                &inshandle
                    .metadata()
                    .dependencies()
                    .iter()
                    .filter(|a| containers.contains(a))
                    .map(|a| *a)
                    .collect(),
            )?;
            self.transact(inshandle)?;
        }

        Ok(())
    }

    pub fn transact(&mut self, inshandle: &'a ContainerHandle) -> Result<()> {
        let queue = match self.pkg_queue.get(inshandle.vars().instance()) {
            Some(some) => some.clone(),
            None => Vec::new(),
        };
        let alpm = sync::instantiate_alpm(&inshandle);
        let mut meta = TransactionMetadata::new(queue);
        let mut handle = TransactionHandle::new(&mut meta).alpm_handle(alpm);
        let mut act: Box<dyn Transaction> = TransactionState::Prepare.from(self);

        self.action.begin_message(&inshandle);

        loop {
            let result = match act.engage(self, &mut handle, inshandle) {
                Ok(result) => {
                    if let TransactionState::Complete(updated) = result {
                        if updated {
                            self.updated.insert(inshandle.vars().instance());
                        }

                        handle.release();
                        return Ok(());
                    } else if let TransactionState::Prepare = result {
                        self.updated.insert(inshandle.vars().instance());
                    }

                    result
                }
                Err(err) => {
                    handle.release();
                    return match err.downcast::<SyncError>() {
                        Ok(error) => match error {
                            SyncError::TransactionFailureAgent => exit(err.kind().code()),
                            SyncError::NothingToDo(bool) => match bool {
                                false => Ok(()),
                                true => Err(err),
                            },
                            _ => Err(err),
                        },
                        Err(_) => err!(SyncError::from(err)),
                    };
                }
            };

            act = result.from(self);
        }
    }

    pub fn keyring_update(&mut self, inshandle: &ContainerHandle) -> Result<()> {
        fakeroot_container(NonInteractive, None, inshandle, vec!["/usr/bin/pacwrap-key", "--populate", "archlinux"])?;
        self.keyring = true;
        Ok(())
    }

    pub fn cache(&self) -> &ContainerCache {
        &self.cache
    }

    pub fn action(&self) -> &TransactionType {
        &self.action
    }

    pub fn updated(&self, inshandle: &ContainerHandle<'a>) -> bool {
        self.updated.contains(&inshandle.vars().instance())
    }

    pub fn is_keyring_synced(&self) -> bool {
        self.keyring
    }

    pub fn flags(&self) -> &TransactionFlags {
        &self.flags
    }

    pub fn logger(&mut self) -> &mut Logger {
        &mut self.logger
    }

    pub fn fs_sync(&mut self) -> Result<&mut FileSystemStateSync<'a>> {
        match self.filesystem_state.as_mut() {
            Some(linker) => Ok(linker),
            None => err!(ErrorKind::LinkerUninitialized),
        }
    }
}

fn signal_trap(cache: &ContainerCache<'_>) {
    let mut signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM]).unwrap();
    let mut paths = vec![format!("{}/pacman/db.lck", *DATA_DIR)];
    let container_vars: Vec<&ContainerHandle> = cache.registered_handles();
    for container in container_vars {
        paths.push(format!("{}/var/lib/pacman/db.lck", container.vars().root()));
    }

    thread::spawn(move || {
        for s in signals.forever() {
            unlock_databases(paths);
            println!("\n{} Transaction interrupted by signal interrupt.", *ARROW_RED);
            exit(128 + s);
        }
    });
}

fn unlock_databases(db_paths: Vec<String>) {
    for path in db_paths {
        if Path::new(&path).exists() {
            fs::remove_file(&path).ok();
        }
    }
}
