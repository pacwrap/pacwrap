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

use std::{borrow::Cow, collections::HashSet};

use alpm::{Alpm, Package, PackageReason, TransFlag};
use bitflags::bitflags;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};

use self::{SyncState::*, TransactionMode::*, TransactionType::*};
use crate::{
    config::{ContainerHandle, Global, CONFIG},
    constants::{ARROW_CYAN, BAR_CYAN, BOLD, BOLD_GREEN, BOLD_YELLOW, RESET},
    err,
    sync::{
        resolver::DependencyResolver,
        resolver_local::LocalDependencyResolver,
        schema::SchemaState,
        transaction::{commit::Commit, container::Schema, prepare::Prepare, stage::Stage, uptodate::UpToDate},
        utils::AlpmUtils,
        SyncError,
    },
    utils::{print_warning, prompt::prompt},
    Error,
};

pub use self::aggregator::TransactionAggregator;

pub mod aggregator;
mod commit;
mod container;
mod prepare;
mod stage;
mod uptodate;

pub type Result<T> = crate::Result<T>;
pub static MAGIC_NUMBER: u32 = 663445956;

pub enum TransactionState {
    Complete(bool),
    Prepare,
    UpToDate,
    PrepareForeign(bool),
    Stage,
    StageForeign,
    UpdateSchema(Option<SchemaState>),
    Commit(bool),
    CommitForeign,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum TransactionType {
    Upgrade(bool, bool, bool),
    Remove(bool, bool, bool),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum TransactionMode {
    Foreign,
    Local,
}

pub enum SyncState {
    Required,
    NotRequired,
}

pub trait Transaction {
    fn new(new: TransactionState, ag: &TransactionAggregator) -> Box<Self>
    where
        Self: Sized;
    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        handle: &mut TransactionHandle,
        inshandle: &ContainerHandle,
    ) -> Result<TransactionState>;
}

bitflags! {
    pub struct TransactionFlags: u8 {
        const NONE = 0;
        const TARGET_ONLY = 0b0000001;
        const PREVIEW = 0b0000010;
        const NO_CONFIRM =  0b0000100;
        const FORCE_DATABASE = 0b0001000;
        const DATABASE_ONLY = 0b0010000;
        const CREATE = 0b0100000;
        const FILESYSTEM_SYNC =  0b1000000;
    }
}

pub struct TransactionHandle<'a> {
    meta: &'a mut TransactionMetadata<'a>,
    fail: bool,
    agent: bool,
    config: Option<&'a Global>,
    alpm: Option<Alpm>,
    deps: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionMetadata<'a> {
    foreign_pkgs: HashSet<String>,
    resident_pkgs: HashSet<String>,
    ignored_pkgs: HashSet<String>,
    held_pkgs: HashSet<String>,
    queue: Vec<Cow<'a, str>>,
    mode: TransactionMode,
    flags: (u8, u32),
}

#[derive(Serialize, Deserialize)]
pub struct TransactionParameters {
    magic: u32,
    ver_major: u8,
    ver_minor: u8,
    ver_patch: u8,
    bytes: u64,
    files: u64,
    action: TransactionType,
    mode: TransactionMode,
}

impl TransactionMode {
    fn bool(&self) -> bool {
        match self {
            Self::Foreign => true,
            Self::Local => false,
        }
    }
}

impl TransactionState {
    fn from(self, ag: &TransactionAggregator) -> Box<dyn Transaction> {
        match self {
            Self::UpdateSchema(_) => Schema::new(self, ag),
            Self::Prepare => Prepare::new(self, ag),
            Self::PrepareForeign(_) => Prepare::new(self, ag),
            Self::UpToDate => UpToDate::new(self, ag),
            Self::Stage => Stage::new(self, ag),
            Self::StageForeign => Stage::new(self, ag),
            Self::Commit(_) => Commit::new(self, ag),
            Self::CommitForeign => Commit::new(self, ag),
            Self::Complete(_) => unreachable!(),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Commit(_) => "resident",
            Self::CommitForeign => "foreign",
            _ => "",
        }
    }
}

impl TransactionType {
    pub fn pr_offset(&self) -> usize {
        match self {
            Self::Upgrade(..) => 1,
            Self::Remove(..) => 0,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Upgrade(..) => "installation",
            Self::Remove(..) => "removal",
        }
    }

    fn action_message(&self, state: TransactionMode) {
        let message = match self {
            Self::Upgrade(..) => match state {
                Foreign => "Synchronizing foreign database...",
                Local => "Synchronizing resident container...",
            },
            Self::Remove(..) => match state {
                Foreign => "Preparing foreign package removal...",
                Local => "Preparing package removal...",
            },
        };

        println!("{} {}", *ARROW_CYAN, message);
    }

    fn begin_message(&self, inshandle: &ContainerHandle, progress: Option<&ProgressBar>) {
        let instance = inshandle.vars().instance();
        let message = match self {
            Self::Upgrade(upgrade, ..) => match upgrade {
                true => format!("Checking {instance} for updates..."),
                false => format!("Transacting {instance}..."),
            },
            Self::Remove(..) => format!("Transacting {instance}..."),
        };

        if let Some(progress) = progress {
            progress.inc(1);
            progress.set_message(format!("{}{}{} ", *BOLD, message, *RESET));
            progress.tick();
        } else {
            println!("{} {}{}{}", *BAR_CYAN, *BOLD, message, *RESET);
        }
    }
}

impl<'a> TransactionMetadata<'a> {
    fn new(queue: Vec<&'a str>) -> TransactionMetadata {
        Self {
            foreign_pkgs: HashSet::new(),
            resident_pkgs: HashSet::new(),
            held_pkgs: HashSet::new(),
            ignored_pkgs: HashSet::new(),
            mode: Local,
            queue: queue.iter().map(|q| (*q).into()).collect::<Vec<_>>(),
            flags: (0, 0),
        }
    }

    pub fn set_flags(&mut self, flags: &TransactionFlags, flags_alpm: &TransFlag) {
        self.flags = (flags.bits(), flags_alpm.bits());
    }

    pub fn retrieve_flags(&self) -> (Option<TransactionFlags>, Option<TransFlag>) {
        (TransactionFlags::from_bits(self.flags.0), TransFlag::from_bits(self.flags.1))
    }
}

impl<'a> TransactionHandle<'a> {
    pub fn new(metadata: &'a mut TransactionMetadata<'a>) -> Self {
        Self {
            meta: metadata,
            fail: true,
            agent: false,
            alpm: None,
            deps: None,
            config: None,
        }
    }

    pub fn config(mut self, config: &'a Global) -> Self {
        self.config = Some(config);
        self
    }

    pub fn alpm_handle(mut self, alpm_handle: Alpm) -> Self {
        self.alpm = Some(alpm_handle);
        self
    }

    pub fn agent(mut self) -> Self {
        self.agent = true;
        self
    }

    fn is_sync_req(&self, mode: TransactionMode) -> SyncState {
        let alpm = self.alpm();
        let ignored = match mode {
            Foreign => &self.meta.resident_pkgs,
            Local => &self.meta.foreign_pkgs,
        };

        for pkg in alpm.localdb().pkgs() {
            if let Some(_) = ignored.get(pkg.name()) {
                continue;
            }

            if let Some(_) = pkg.sync_new_version(alpm.syncdbs()) {
                return Required;
            }
        }

        NotRequired
    }

    fn enumerate_package_lists(&mut self, dep_handle: &Alpm) {
        self.meta.foreign_pkgs.extend(
            dep_handle
                .localdb()
                .pkgs()
                .iter()
                .filter(|p| !self.meta.foreign_pkgs.contains(p.name()))
                .map(|p| p.name().into())
                .collect::<Vec<_>>(),
        );
        self.meta.resident_pkgs.extend(
            self.alpm()
                .localdb()
                .pkgs()
                .iter()
                .filter(|p| !self.meta.foreign_pkgs.contains(p.name()) && !self.meta.resident_pkgs.contains(p.name()))
                .map(|a| a.name().into())
                .collect::<Vec<_>>(),
        );
    }

    fn enumerate_foreign_queue(&mut self) {
        self.meta
            .queue
            .extend(self.meta.foreign_pkgs.iter().map(|p| p.to_owned().into()).collect::<Vec<_>>());
        self.fail = false;
    }

    pub fn ignore(&mut self) {
        let mut fail = self.fail;
        let alpm = self.alpm.as_mut().unwrap();
        let config = match self.config {
            Some(config) => config,
            None => &*CONFIG,
        };
        let ignore = match self.meta.mode {
            Foreign => &self.meta.resident_pkgs,
            Local => &self.meta.foreign_pkgs,
        };
        let unignore = match self.meta.mode {
            Local => &self.meta.resident_pkgs,
            Foreign => &self.meta.foreign_pkgs,
        };

        for pkg in unignore {
            alpm.remove_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for pkg in ignore {
            alpm.add_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for pkg in config.alpm().ignored() {
            alpm.add_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for package in alpm
            .localdb()
            .pkgs()
            .iter()
            .filter(|a| !ignore.contains(a.name()) && config.alpm().ignored().contains(&a.name()))
        {
            let new = match package.sync_new_version(alpm.syncdbs()) {
                Some(new) => {
                    fail = false;

                    match self.agent {
                        true => break,
                        false => new,
                    }
                }
                None => continue,
            };
            let name = package.name();
            let ver = package.version();
            let ver_new = new.version();

            print_warning(&format!(
                "{}{name}{}: Ignoring package upgrade ({}{ver}{} => {}{ver_new}{})",
                *BOLD, *RESET, *BOLD_YELLOW, *RESET, *BOLD_GREEN, *RESET
            ));
        }

        self.fail = fail;
    }

    pub fn prepare(&mut self, trans_type: &TransactionType, flags: &TransactionFlags) -> Result<()> {
        let alpm = self.alpm.as_mut().unwrap();
        let ignored = match self.meta.mode {
            Foreign => &self.meta.resident_pkgs,
            Local => &self.meta.foreign_pkgs,
        };
        let queue = self.meta.queue.iter().map(|i| i.as_ref()).collect::<Vec<_>>();
        let config = match self.config {
            Some(config) => config,
            None => &*CONFIG,
        };

        if let Local = self.meta.mode {
            let upstream = queue.iter().map(|a| *a).find(|a| ignored.contains(*a));
            let forced = flags.contains(TransactionFlags::FORCE_DATABASE);

            if let (false, Some(upstream)) = (forced, upstream) {
                err!(SyncError::TargetUpstream(upstream.into()))?
            }
        }

        match trans_type {
            Remove(..) => {
                if let Some(not_installed) = queue
                    .iter()
                    .map(|a| *a)
                    .find(|a| !ignored.contains(*a) && alpm.get_local_package(a).is_none())
                {
                    err!(SyncError::TargetNotInstalled(not_installed.into()))?
                }

                for pkg in LocalDependencyResolver::new(alpm, &ignored, trans_type)
                    .enumerate(&queue)?
                    .iter()
                    .filter(|a| !self.meta.held_pkgs.contains(a.name()))
                    .map(|a| *a)
                    .collect::<Vec<&Package>>()
                {
                    if !self.agent && !ignored.contains(pkg.name()) && config.alpm().held().contains(&pkg.name()) {
                        if let Err(_) =
                            prompt("::", format!("Target package {}{}{} is held. Remove it?", *BOLD, pkg.name(), *RESET), false)
                        {
                            self.meta.held_pkgs.insert(pkg.name().into());
                            continue;
                        }
                    }

                    alpm.trans_remove_pkg(pkg).unwrap();
                }
            }
            Upgrade(..) => {
                if let Some(not_available) = queue.iter().map(|a| *a).find(|a| alpm.get_package(a).is_none()) {
                    err!(SyncError::TargetNotAvailable(not_available.into()))?
                }

                let (deps, packages) = DependencyResolver::new(alpm, &ignored).enumerate(&queue)?;

                for pkg in packages
                    .iter()
                    .filter(|a| !self.meta.ignored_pkgs.contains(a.name()))
                    .filter_map(|a| {
                        if let (None, Foreign) = (self.meta.foreign_pkgs.get(a.name()), self.meta.mode) {
                            None
                        } else {
                            Some(*a)
                        }
                    })
                    .collect::<Vec<&Package>>()
                {
                    if !self.agent && config.alpm().ignored().contains(&pkg.name()) {
                        if let Err(_) = prompt(
                            "::",
                            format!("Target package {}{}{} is ignored. Upgrade it?", *BOLD, pkg.name(), *RESET),
                            false,
                        ) {
                            self.meta.ignored_pkgs.insert(pkg.name().into());
                            continue;
                        }
                    }

                    alpm.trans_add_pkg(pkg).unwrap();
                }

                self.deps = deps;
            }
        }

        Ok(())
    }

    fn apply_configuration(&mut self, instance: &ContainerHandle, create: bool) -> Result<()> {
        let depends = instance.metadata().dependencies();
        let explicit_packages: Vec<&str> = instance.metadata().explicit_packages();
        let pkgs = self
            .alpm
            .as_mut()
            .unwrap()
            .localdb()
            .pkgs()
            .iter()
            .filter(|p| {
                p.reason() == PackageReason::Explicit
                    && !p.name().starts_with("pacwrap-")
                    && !self.meta.foreign_pkgs.contains(p.name())
            })
            .map(|p| p.name())
            .collect();

        if pkgs != explicit_packages || create {
            let mut instance = instance.clone();

            instance.metadata_mut().set_metadata(depends, pkgs);
            instance.save()?;
            drop(instance);
        }

        Ok(())
    }

    pub fn trans_ready(&mut self, trans_type: &TransactionType) -> Result<()> {
        if match trans_type {
            Upgrade(..) => self.alpm().trans_add().len(),
            Remove(..) => self.alpm().trans_remove().len(),
        } > 0
        {
            Ok(())
        } else {
            err!(SyncError::NothingToDo(self.fail))
        }
    }

    pub fn mark_depends(&mut self) {
        if let Some(deps) = self.deps.as_ref() {
            for pkg in deps.iter().filter_map(|a| self.alpm().get_local_package(a)) {
                pkg.set_reason(PackageReason::Depend).unwrap();
            }
        }
    }

    pub fn release(mut self) {
        if let Some(alpm) = self.alpm.as_mut() {
            alpm.trans_release().ok();
        }

        drop(self);
    }

    fn set_mode(&mut self, modeset: TransactionMode) {
        self.meta.mode = modeset;
    }

    pub fn get_mode(&self) -> &TransactionMode {
        &self.meta.mode
    }

    pub fn alpm_mut(&mut self) -> &mut Alpm {
        self.alpm.as_mut().unwrap()
    }

    pub fn alpm(&self) -> &Alpm {
        self.alpm.as_ref().unwrap()
    }

    pub fn set_alpm(&mut self, alpm: Option<Alpm>) {
        self.alpm = alpm;
    }

    pub fn metadata(&self) -> &TransactionMetadata {
        &self.meta
    }
}

impl TransactionParameters {
    fn new(t_type: TransactionType, t_mode: TransactionMode, download: (u64, u64)) -> Self {
        Self {
            magic: MAGIC_NUMBER,
            ver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            ver_minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            ver_patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
            bytes: download.0,
            files: download.1,
            action: t_type,
            mode: t_mode,
        }
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn files(&self) -> usize {
        self.files as usize
    }

    pub fn mode(&self) -> TransactionMode {
        self.mode
    }

    pub fn action(&self) -> TransactionType {
        self.action
    }
}
