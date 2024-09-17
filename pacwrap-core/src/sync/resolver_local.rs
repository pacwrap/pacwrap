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

use std::collections::HashSet;

use alpm::{Alpm, Package, PackageReason};

use crate::{
    err,
    sync::{transaction::TransactionType, utils::AlpmUtils, SyncError},
    Error,
};

pub struct LocalDependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<&'a Package>,
    ignored: &'a HashSet<String>,
    handle: &'a Alpm,
    depth: isize,
    flags: (bool, bool, bool),
}

impl<'a> LocalDependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a HashSet<String>, trans_type: &TransactionType) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
            flags: match trans_type {
                TransactionType::Remove(enumerate, cascade, explicit) => (*enumerate, *cascade, *explicit),
                _ => panic!("Invalid transaction type for this resolver."),
            },
        }
    }

    fn check_depth(&mut self) -> Result<(), Error> {
        if self.depth == 50 {
            err!(SyncError::RecursionDepthExceeded(self.depth))?
        }

        self.depth += 1;
        Ok(())
    }

    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Result<Vec<&'a Package>, Error> {
        let mut synchronize: Vec<&'a str> = Vec::new();

        for pkg in packages {
            if self.resolved.contains(*pkg) {
                continue;
            }

            if self.ignored.contains(*pkg) {
                continue;
            }

            if let Some(pkg) = self.handle.get_local_package(pkg) {
                if self.depth > 0 {
                    //TODO: Implement proper explicit package handling
                    if !self.flags.1 && pkg.reason() == PackageReason::Explicit {
                        continue;
                    }

                    if pkg.required_by().iter().any(|p| self.resolved.contains(p)) {
                        continue;
                    }
                }

                self.packages.push(pkg);
                self.resolved.insert(pkg.name());

                if !self.flags.0 {
                    continue;
                }

                synchronize.extend(pkg.depends().iter().map(|pkg| pkg.name()).collect::<Vec<&str>>());

                if !self.flags.1 {
                    continue;
                }

                for package in self.handle.localdb().pkgs() {
                    if package.depends().iter().find_map(|d| self.resolved.get(d.name())).is_some() {
                        synchronize.push(package.name());
                    }
                }
            }
        }

        if !synchronize.is_empty() && self.flags.0 {
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
            Ok(self.packages)
        }
    }
}
