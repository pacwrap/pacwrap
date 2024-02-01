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

use std::{collections::HashMap, process::exit};

use crate::{
    config::{cache::InstanceCache, InstanceHandle, InstanceType, CONFIG},
    constants::{ARROW_GREEN, UNIX_TIMESTAMP},
    err,
    error::*,
    exec::{fakeroot_container, ExecutionType::NonInteractive},
    log::Logger,
    sync::{
        self,
        filesystem::FileSystemStateSync,
        transaction::{Transaction, TransactionFlags, TransactionHandle, TransactionMetadata, TransactionState, TransactionType},
        SyncError,
    },
    ErrorKind,
};

pub struct TransactionAggregator<'a> {
    queried: Vec<&'a str>,
    updated: Vec<&'a str>,
    pkg_queue: HashMap<&'a str, Vec<&'a str>>,
    action: TransactionType,
    filesystem_state: Option<FileSystemStateSync<'a>>,
    cache: &'a InstanceCache<'a>,
    keyring: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags,
    target: Option<&'a str>,
}

impl<'a> TransactionAggregator<'a> {
    pub fn new(
        inscache: &'a InstanceCache,
        queue: HashMap<&'a str, Vec<&'a str>>,
        log: &'a mut Logger,
        action_flags: TransactionFlags,
        action_type: TransactionType,
        current_target: Option<&'a str>,
    ) -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            pkg_queue: queue,
            filesystem_state: Some(FileSystemStateSync::new(inscache)),
            action: action_type,
            cache: inscache,
            keyring: false,
            logger: log,
            flags: action_flags,
            target: current_target,
        }
    }

    pub fn aggregate(mut self) -> Result<()> {
        let _timestamp = *UNIX_TIMESTAMP;
        let upgrade = match self.action {
            TransactionType::Upgrade(upgrade, refresh, force) => {
                if refresh {
                    sync::synchronize_database(self.cache, force)?;
                }

                upgrade
            }
            _ => false,
        };
        let target = match self.target {
            Some(s) => self.cache.get_instance_option(s),
            None => None,
        };
        let downstream = self.cache.filter(vec![InstanceType::Aggregate]);
        let upstream = self.cache.filter(vec![InstanceType::Base, InstanceType::Slice]);
        let containers = (upstream, downstream);

        if let Some(ins) = target {
            if let InstanceType::Base | InstanceType::Slice = ins.metadata().container_type() {
                self.transact(ins)?;
            }
        } else if upgrade {
            self.transaction(&containers.0)?;
        }

        if self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC | TransactionFlags::CREATE) || self.updated.len() > 0 {
            if containers.1.len() > 0 {
                let file = self.cache.registered();
                let linker = self.fs_sync().unwrap();

                linker.prepare(file.len());
                linker.engage(&file)?;
                linker.finish();
            }

            self.filesystem_state = self.filesystem_state.unwrap().release();
        }

        if let Some(ins) = target {
            if let InstanceType::Aggregate = ins.metadata().container_type() {
                self.transact(ins)?;
            }
        } else if upgrade {
            self.transaction(&containers.1)?;
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

            self.queried.push(ins);
            self.transaction(&inshandle.metadata().dependencies())?;
            self.transact(inshandle)?;
        }

        Ok(())
    }

    pub fn transact(&mut self, inshandle: &'a InstanceHandle) -> Result<()> {
        let queue = match self.pkg_queue.get(inshandle.vars().instance()) {
            Some(some) => some.clone(),
            None => Vec::new(),
        };
        let alpm = sync::instantiate_alpm(&inshandle);
        let mut meta = TransactionMetadata::new(queue);
        let mut handle = TransactionHandle::new(&*CONFIG, alpm, &mut meta);
        let mut act: Box<dyn Transaction> = TransactionState::Prepare.from(self);

        self.action.begin_message(&inshandle);

        loop {
            let result = match act.engage(self, &mut handle, inshandle) {
                Ok(result) => {
                    if let TransactionState::Complete(updated) = result {
                        if updated {
                            self.updated.push(inshandle.vars().instance());
                        }

                        handle.release();
                        return Ok(());
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

    pub fn keyring_update(&mut self, inshandle: &InstanceHandle) -> Result<()> {
        fakeroot_container(NonInteractive, None, inshandle, vec!["/usr/bin/pacman-key", "--populate", "archlinux"])?;
        fakeroot_container(NonInteractive, None, inshandle, vec!["/usr/bin/pacman-key", "--updatedb"])?;
        self.keyring = true;
        Ok(())
    }

    pub fn sync_filesystem(&mut self, inshandle: &'a InstanceHandle) -> Result<()> {
        if let InstanceType::Aggregate = inshandle.metadata().container_type() {
            return Ok(());
        }

        let fs_sync = self.fs_sync().unwrap();

        fs_sync.prepare_single();
        fs_sync.engage(&vec![inshandle.vars().instance()])
    }

    pub fn cache(&self) -> &InstanceCache {
        &self.cache
    }

    pub fn action(&self) -> &TransactionType {
        &self.action
    }

    pub fn deps_updated(&self, inshandle: &InstanceHandle<'a>) -> bool {
        for ins in inshandle.metadata().dependencies() {
            if self.updated.contains(&ins) {
                return true;
            }
        }

        false
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
