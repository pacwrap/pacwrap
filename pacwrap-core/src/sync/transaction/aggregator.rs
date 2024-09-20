/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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

use std::collections::{HashMap, HashSet};

use alpm::Alpm;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use lazy_static::lazy_static;
use signal_hook::iterator::Signals;

use crate::{
    config::{cache::ContainerCache, ContainerHandle, ContainerType::*},
    constants::{ARROW_GREEN, IS_COLOR_TERMINAL, SIGNAL_LIST, UNIX_TIMESTAMP, VERBOSE},
    err,
    error,
    exec::{fakeroot_container, ExecutionType::NonInteractive},
    lock::{Lock, LockError},
    log::{Level, Logger},
    sync::{
        self,
        filesystem::{validate_fs_states, FilesystemSync},
        transaction::{
            Transaction,
            TransactionFlags,
            TransactionHandle,
            TransactionMetadata,
            TransactionState::*,
            TransactionType::{self, *},
        },
        utils::signal_trap,
        SyncError,
    },
    utils::arguments::InvalidArgument,
    Error,
    Result,
};

lazy_static! {
    pub static ref BAR_CYAN_STYLE: ProgressStyle = ProgressStyle::with_template("{spinner:.bold.cyan} {msg}")
        .unwrap()
        .tick_strings(&["::", ":.", ".:", "::"]);
    pub static ref BAR_GREEN_STYLE: ProgressStyle = ProgressStyle::with_template("{spinner:.bold.green} {msg}")
        .unwrap()
        .tick_strings(&["::", ":.", ".:", "::"]);
}

pub struct TransactionAggregator<'a> {
    queried: HashSet<&'a str>,
    updated: HashSet<&'a str>,
    pkg_queue: HashMap<&'a str, Vec<&'a str>>,
    action: TransactionType,
    cache: &'a ContainerCache<'a>,
    keyring: bool,
    tracted: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags,
    targets: Option<Vec<&'a str>>,
    lock: Option<&'a Lock>,
    progress: Option<ProgressBar>,
    signals: Signals,
}

impl<'a> TransactionAggregator<'a> {
    pub fn new(inscache: &'a ContainerCache, log: &'a mut Logger, action_type: TransactionType) -> Self {
        Self {
            targets: None,
            queried: HashSet::new(),
            updated: HashSet::new(),
            pkg_queue: HashMap::new(),
            action: action_type,
            cache: inscache,
            keyring: false,
            tracted: false,
            logger: log,
            flags: TransactionFlags::NONE,
            lock: None,
            progress: None,
            signals: Signals::new(SIGNAL_LIST).unwrap(),
        }
    }

    pub fn progress(mut self) -> Self {
        if let (.., Remove(..)) | (.., Upgrade(false, true, ..)) | (false, ..) = (*IS_COLOR_TERMINAL && !*VERBOSE, self.action) {
            return self;
        }

        self.progress = Some(ProgressBar::new_spinner().with_style(BAR_CYAN_STYLE.clone()));
        self
    }

    pub fn flag(mut self, flags: TransactionFlags) -> Self {
        if flags.intersects(TransactionFlags::DEBUG) {
            self.logger.set_verbosity(4);
        }

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

    pub fn assert_lock(mut self, lock: &'a Lock) -> Result<Self> {
        lock.assert()?;
        self.lock = Some(lock);
        Ok(self)
    }

    pub fn aggregate(mut self) -> Result<()> {
        self.lock()?;
        signal_trap();

        let _timestamp = *UNIX_TIMESTAMP;
        let preview = self.flags.intersects(TransactionFlags::PREVIEW);
        let filesystem_sync = self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC | TransactionFlags::CREATE);
        let transact = match self.action {
            Upgrade(upgrade, refresh, force) => {
                if !upgrade
                    && !refresh
                    && !self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC | TransactionFlags::TARGET_ONLY)
                {
                    err!(InvalidArgument::OperationUnspecified)?
                }

                if refresh {
                    sync::synchronize_database(&mut self, force)?;
                }

                upgrade | self.targets.is_some()
            }
            Remove(..) => self.targets.is_some(),
        };
        let upstream = match self.targets.as_ref() {
            Some(targets) => self.cache.filter_target(targets, vec![Base, Slice]),
            None => self.cache.filter(vec![Base, Slice]),
        };
        let downstream = match self.targets.as_ref() {
            Some(targets) => self.cache.filter_target(targets, vec![Aggregate]),
            None => self.cache.filter(vec![Aggregate]),
        };
        let are_downstream = self.cache.count(vec![Aggregate]) > 0;
        let target_amount = (downstream.len() + upstream.len()) as u64;
        let mut linker = FilesystemSync::new(self.cache).assert_lock(self.lock);

        if upstream.is_empty() && downstream.is_empty() {
            err!(SyncError::NothingToDo)?
        }

        if let Some(progress) = self.progress.as_ref() {
            progress.set_draw_target(ProgressDrawTarget::stderr());
            progress.set_length(target_amount);
        }

        if !validate_fs_states(&upstream) && !preview && are_downstream {
            linker.refresh_state();
            linker.prepare(upstream.len(), self.progress.as_ref());
            linker.engage(&upstream)?;
            linker.finish(self.progress.as_ref());
        }

        if transact {
            self.transaction(&upstream)?;
        }

        if are_downstream {
            if !preview && (filesystem_sync || !self.updated.is_empty()) {
                linker.filesystem_state();
                linker.prepare(self.cache.registered().len(), self.progress.as_ref());
                linker.engage(&self.cache.registered())?;
                linker.finish(self.progress.as_ref());
            }

            linker.release();
        }

        if transact {
            self.transaction(&downstream)?;
        }

        self.print_complete(filesystem_sync, target_amount, upstream.last().or_else(|| downstream.last()));
        Ok(())
    }

    pub fn transaction(&mut self, containers: &[&'a str]) -> Result<()> {
        for ins in containers.iter() {
            if self.queried.contains(ins) {
                continue;
            }

            let inshandle = match self.cache.get_instance_option(ins) {
                Some(ins) => ins,
                None => continue,
            };

            self.signal(&mut None)?;
            self.queried.insert(ins);
            self.transaction(
                &inshandle
                    .metadata()
                    .dependencies()
                    .iter()
                    .filter(|a| containers.contains(a))
                    .copied()
                    .collect::<Vec<&str>>(),
            )?;
            self.transact(inshandle)?;
        }

        Ok(())
    }

    fn transact(&mut self, inshandle: &'a ContainerHandle) -> Result<()> {
        if let Err(err) = self.lock()?.assert() {
            err!(SyncError::from(&err))?
        }

        let queue = match self.pkg_queue.get(inshandle.vars().instance()) {
            Some(some) => some.clone(),
            None => Vec::new(),
        };

        let alpm = sync::instantiate_alpm(inshandle, self.flags())?;
        let mut meta = TransactionMetadata::new(queue);
        let mut handle = TransactionHandle::new(&mut meta).alpm_handle(alpm);
        let mut act: Box<dyn Transaction> = Prepare.from(self);

        self.signal(&mut handle.alpm)?;
        self.action().begin_message(inshandle, self.progress.as_ref());

        loop {
            self.logger().log(Level::Debug, &format!("Transaction state: {}", act.debug()))?;
            act = match act.engage(self, &mut handle, inshandle) {
                Ok(state) => {
                    self.signal(&mut handle.alpm)?;

                    if let Skip = state {
                        self.logger().log(Level::Debug, &format!("Transaction state: {}", act.debug()))?;
                        handle.release();
                        return Ok(());
                    } else if let Complete(updated) = state {
                        if updated {
                            self.updated.insert(inshandle.vars().instance());

                            if self.progress.is_some() {
                                println!();
                            }
                        }

                        self.logger().log(Level::Debug, &format!("Transaction state: {}", act.debug()))?;
                        self.tracted = !updated;
                        handle.release();
                        return Ok(());
                    } else if let UpdateSchema(_) = state {
                        self.updated.insert(inshandle.vars().instance());
                    }

                    state
                }
                Err(err) => {
                    if let Some(progress) = self.progress.as_ref() {
                        progress.set_draw_target(ProgressDrawTarget::hidden());
                        progress.finish();
                    }

                    handle.release();
                    return match err.downcast::<SyncError>().map_err(|err| error!(SyncError::from(err)))? {
                        SyncError::TransactionAgentFailure => {
                            self.logger().log(Level::Fatal, &format!("Transaction error: {:?}", err))?;
                            err.fatal()
                        }
                        _ => {
                            self.logger().log(Level::Error, &format!("Transaction error: {:?}", err))?;
                            Err(err)
                        }
                    };
                }
            }
            .from(self);
        }
    }

    fn print_complete(&mut self, filesystem_sync: bool, target_amount: u64, target: Option<&&str>) {
        if self.progress.is_some() {
            let are_multiple = target_amount > 1;
            let flagged = self.flags.intersects(TransactionFlags::PREVIEW | TransactionFlags::CREATE);
            let container = if filesystem_sync && self.queried.is_empty() || flagged || self.tracted {
                None
            } else if are_multiple {
                Some("Containers")
            } else {
                target.copied()
            };
            let message = if self.updated.is_empty() {
                container.map_or_else(
                    || "Transaction complete.".to_string(),
                    |c| format!("{} {} up-to-date.", c, if are_multiple { "are" } else { "is" }),
                )
            } else {
                "Transaction complete.".to_string()
            };

            println!("{} {}", *ARROW_GREEN, message);
        } else {
            println!("{} Transaction complete.", *ARROW_GREEN);
        }
    }

    fn signal(&mut self, handle: &mut Option<Alpm>) -> Result<()> {
        for _ in self.signals.pending() {
            if let Some(handle) = handle {
                handle.trans_interrupt().ok();
            }

            err!(SyncError::SignalInterrupt)?;
        }

        Ok(())
    }

    pub fn keyring_update(&mut self, inshandle: &ContainerHandle) -> Result<()> {
        fakeroot_container(NonInteractive, None, inshandle, vec!["/usr/bin/pacwrap-key", "--populate", "archlinux"])?;
        self.keyring = true;
        Ok(())
    }

    pub fn lock(&mut self) -> Result<&Lock> {
        self.lock.map_or_else(|| err!(LockError::NotAcquired), Ok)
    }

    pub fn cache(&self) -> &ContainerCache {
        self.cache
    }

    pub fn action(&self) -> &TransactionType {
        &self.action
    }

    pub fn progress_bar(&self) -> Option<&ProgressBar> {
        self.progress.as_ref()
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
        self.logger
    }
}
