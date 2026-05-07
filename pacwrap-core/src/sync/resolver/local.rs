/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2025 Xavier Moffett <sapphirus@azorium.net>
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
    Result,
    sync::{
        resolver::{Resolver, ResolverDepth, ResolverDepthExt, ResolverFlags},
        transaction::TransactionType,
        utils::AlpmUtils,
    },
};

pub struct LocalDependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<&'a Package>,
    ignored: Option<&'a HashSet<String>>,
    flags: Option<(bool, bool, bool)>,
    handle: &'a Alpm,
    depth: isize,
}

impl<'a> Resolver<'a> for LocalDependencyResolver<'a> {
    fn new(alpm: &'a Alpm) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            depth: 0,
            handle: alpm,
            ignored: None,
            flags: None,
        }
    }

    fn set_ignored(mut self, ignorelist: &'a HashSet<String>) -> Self {
        self.ignored = Some(ignorelist);
        self
    }

    fn enumerate(mut self, packages: &[&'a str]) -> Result<Self> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        let (enumerate, cascade, _) = self.flags.expect("Transaction flags not set");
        let ignored = self.ignored.expect("Ignore list not enumerated");

        for pkg in packages {
            if self.resolved.contains(*pkg) {
                continue;
            }

            if ignored.contains(*pkg) {
                continue;
            }

            if let Some(pkg) = self.handle.get_local_package(pkg) {
                if self.depth > 0 {
                    //TODO: Implement proper explicit package handling
                    if !cascade && pkg.reason() == PackageReason::Explicit {
                        continue;
                    }

                    if pkg.required_by().iter().any(|p| self.resolved.contains(p)) {
                        continue;
                    }
                }

                self.packages.push(pkg);
                self.resolved.insert(pkg.name());

                if !enumerate {
                    continue;
                }

                synchronize.extend(pkg.depends().iter().map(|pkg| pkg.name()).collect::<Vec<&str>>());

                if !cascade {
                    continue;
                }

                for package in self.handle.localdb().pkgs() {
                    if package.depends().iter().find_map(|d| self.resolved.get(d.name())).is_some() {
                        synchronize.push(package.name());
                    }
                }
            }
        }

        if !synchronize.is_empty() && enumerate {
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
            Ok(self)
        }
    }

    fn packages(&self) -> &Vec<&'a Package> {
        self.packages.as_ref()
    }
}

impl<'a> ResolverFlags for LocalDependencyResolver<'a> {
    fn set_flags(mut self, trans_type: &TransactionType) -> Self {
        self.flags = Some(match trans_type {
            TransactionType::Remove(enumerate, cascade, explicit) => (*enumerate, *cascade, *explicit),
            _ => panic!("Invalid transaction type for this resolver."),
        });
        self
    }
}

impl<'a> ResolverDepthExt for LocalDependencyResolver<'a> {
    fn depth(&self) -> isize {
        self.depth
    }

    fn set_depth(&mut self, depth: isize) {
        self.depth = depth;
    }
}
