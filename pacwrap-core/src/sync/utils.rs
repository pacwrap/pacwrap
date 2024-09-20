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

use std::thread::Builder;

use alpm::{
    Alpm,
    CommitData,
    CommitError,
    Error::{ConflictingDeps, FileConflicts, PkgInvalid, PkgInvalidArch, PkgInvalidChecksum, PkgInvalidSig, UnsatisfiedDeps},
    Package,
    PrepareData,
    PrepareError,
};
use signal_hook::iterator::Signals;

use crate::{
    constants::{BOLD, BOLD_WHITE, RESET, SIGNAL_LIST},
    err,
    error,
    sync::SyncError,
    utils::{print_error, print_warning},
    Error,
    Result,
};

pub trait AlpmUtils {
    fn get_local_package(&self, pkg: &str) -> Option<&Package>;
    fn get_package(&self, pkg: &str) -> Option<&Package>;
}

impl AlpmUtils for Alpm {
    fn get_local_package(&self, pkg: &str) -> Option<&Package> {
        match self.localdb().pkg(pkg) {
            Ok(pkg) => Some(pkg),
            Err(_) => self
                .localdb()
                .pkgs()
                .iter()
                .find_map(|f| f.provides().iter().find(|d| pkg == d.name()).map(|_| f)),
        }
    }

    fn get_package(&self, pkg: &str) -> Option<&Package> {
        for sync in self.syncdbs() {
            if let Ok(pkg) = sync.pkg(pkg) {
                return Some(pkg);
            } else {
                let package = sync.pkgs().iter().find_map(|f| f.provides().iter().find(|d| pkg == d.name()).map(|_| f));

                if package.is_none() {
                    continue;
                }

                return package;
            }
        }

        None
    }
}

pub fn erroneous_transaction(error: CommitError) -> Result<()> {
    /*
     * Qualify error type to ensure no segfault for error conditions of which are
     * unhandled by the upstream data function provided by the CommitError impl.
     *
     * TODO: Possibly submit PR upstream to provide Option encapsulate with None
     * instead of hint::unreachable_unchecked?
     *
     * Then the following block of code could be structured like:
     *
     * ```
     * if let Some(data) = error.data() {
     *  match data {
     *      _ => ()
     *  }
     * }
     * ```
     */
    if let PkgInvalid | PkgInvalidSig | PkgInvalidChecksum | FileConflicts = error.error() {
        match error.data() {
            CommitData::FileConflict(file) => {
                for conflict in file {
                    print_warning(&format!(
                        "Conflict between {}{}{} and {}{}{}: {}",
                        *BOLD,
                        conflict.package1().name(),
                        *RESET,
                        *BOLD,
                        conflict.package2().name(),
                        *RESET,
                        conflict.reason()
                    ));
                }

                err!(SyncError::TransactionFailure("Conflict within container filesystem".into()))?
            }
            CommitData::PkgInvalid(p) =>
                for pkg in p.iter() {
                    print_error(&format!("Invalid package: {}{}{}", *BOLD_WHITE, pkg, *RESET));
                },
        }
    }

    err!(SyncError::TransactionFailure(error.to_string()))
}

pub fn erroneous_preparation(error: PrepareError) -> Result<()> {
    /*
     * Qualify error type to ensure no segfault for error conditions of which are
     * unhandled by the upstream data function provided by the PrepareError impl.
     *
     * TODO: Possibly submit PR upstream to provide Option encapsulate with None
     * instead of hint::unreachable_unchecked?
     *
     * Then the following block of code could be structured like:
     *
     * ```
     * if let Some(data) = error.data() {
     *  match data {
     *      _ => ()
     *  }
     * }
     * ```
     */
    if let PkgInvalidArch | UnsatisfiedDeps | ConflictingDeps = error.error() {
        match error.data() {
            PrepareData::PkgInvalidArch(list) =>
                for package in list.iter() {
                    print_error(&format!(
                        "Invalid architecture {}{}{} for {}{}{}",
                        *BOLD,
                        package.arch().unwrap_or("UNKNOWN"),
                        *RESET,
                        *BOLD,
                        package.name(),
                        *RESET
                    ));
                },
            PrepareData::UnsatisfiedDeps(list) =>
                for missing in list.iter() {
                    print_error(&format!(
                        "Unsatisifed dependency {}{}{} for target {}{}{}",
                        *BOLD,
                        missing.depend(),
                        *RESET,
                        *BOLD,
                        missing.target(),
                        *RESET
                    ));
                },
            PrepareData::ConflictingDeps(list) =>
                for conflict in list.iter() {
                    print_error(&format!(
                        "Conflict between {}{}{} and {}{}{}: {}",
                        *BOLD,
                        conflict.package1().name(),
                        *RESET,
                        *BOLD,
                        conflict.package2().name(),
                        *RESET,
                        conflict.reason()
                    ));
                },
        }
    }

    err!(SyncError::PreparationFailure(error.to_string()))
}

pub fn signal_trap() {
    let mut signals = Signals::new(*SIGNAL_LIST).unwrap();
    let mut count = 0;

    Builder::new()
        .name("pacwrap-signal".to_string())
        .spawn(move || {
            for _ in signals.forever() {
                count += 1;
                println!();

                if count == 3 {
                    error!(SyncError::SignalInterrupt).error()
                }
            }
        })
        .unwrap();
}
