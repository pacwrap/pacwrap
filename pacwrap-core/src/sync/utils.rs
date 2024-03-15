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

use alpm::{Alpm, CommitData, CommitError, Package, PrepareData, PrepareError};

use crate::{
    constants::{BOLD, BOLD_WHITE, RESET},
    err,
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
    fn get_local_package<'a>(&self, pkg: &'a str) -> Option<&Package> {
        if let Ok(pkg) = self.localdb().pkg(pkg) {
            return Some(pkg);
        } else {
            self.localdb()
                .pkgs()
                .iter()
                .find_map(|f| f.provides().iter().find(|d| pkg == d.name()).and_then(|_| Some(f)))
        }
    }

    fn get_package(&self, pkg: &str) -> Option<&Package> {
        for sync in self.syncdbs() {
            if let Ok(pkg) = sync.pkg(pkg) {
                return Some(pkg);
            } else {
                let package = sync
                    .pkgs()
                    .iter()
                    .find_map(|f| f.provides().iter().find(|d| pkg == d.name()).and_then(|_| Some(f)));

                if let None = package {
                    continue;
                }

                return package;
            }
        }

        None
    }
}

pub fn erroneous_transaction<'a>(error: CommitError) -> Result<()> {
    match error.data() {
        CommitData::FileConflict(file) => {
            for conflict in file {
                let reason = conflict.reason();
                let package1 = conflict.package1().name();
                let package2 = conflict.package2().name();

                print_warning(&format!(
                    "Conflict between {}{}{} and {}{}{}: {}",
                    *BOLD, package1, *RESET, *BOLD, package2, *RESET, reason
                ));
            }

            err!(SyncError::TransactionFailure("Conflict within container filesystem".into()))?
        }
        CommitData::PkgInvalid(p) =>
            for pkg in p.iter() {
                let pkg = format!("{}{pkg}{}", *BOLD_WHITE, *RESET);
                print_error(&format!("Invalid package: {}", pkg));
            },
    }

    err!(SyncError::TransactionFailure(error.to_string()))
}

pub fn erroneous_preparation<'a>(error: PrepareError) -> Result<()> {
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

    err!(SyncError::PreparationFailure(error.to_string()))
}
