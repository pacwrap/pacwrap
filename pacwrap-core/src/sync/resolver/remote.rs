/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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

use alpm::{Alpm, Package};

use crate::{
    Result,
    sync::{
        resolver::{Resolver, ResolverDepth, ResolverDepthExt, ResolverKeys},
        utils::AlpmUtils,
    },
};

pub struct DependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<&'a Package>,
    keys: Vec<&'a str>,
    ignored: Option<&'a HashSet<String>>,
    handle: &'a Alpm,
    depth: isize,
}

impl<'a> Resolver<'a> for DependencyResolver<'a> {
    fn new(alpm: &'a Alpm) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            keys: Vec::new(),
            ignored: None,
            depth: 0,
            handle: alpm,
        }
    }

    fn set_ignored(mut self, ignorelist: &'a HashSet<String>) -> Self {
        self.ignored = Some(ignorelist);
        self
    }

    fn enumerate(mut self, packages: &[&'a str]) -> Result<Self> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        let ignored = self.ignored.expect("Ignore list not enumerated");

        for pkg in packages {
            if self.resolved.contains(*pkg) {
                continue;
            }

            if ignored.contains(*pkg) {
                continue;
            }

            if let Some(pkg) = self.handle.get_package(pkg) {
                self.packages.push(pkg);
                self.resolved.insert(pkg.name());
                synchronize.extend(
                    pkg.depends()
                        .iter()
                        .filter_map(|p| match self.handle.get_local_package(p.name()) {
                            None => self.handle.get_package(p.name()).map(|dep| dep.name()),
                            Some(_) => None,
                        })
                        .collect::<Vec<&str>>(),
                );

                if self.depth > 0 {
                    self.keys.push(pkg.name());
                }
            }
        }

        if !synchronize.is_empty() {
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

impl<'a> ResolverKeys for DependencyResolver<'a> {
    fn keys(&self) -> Option<Vec<String>> {
        if !self.keys.is_empty() {
            Some(self.keys.iter().map(|a| (*a).into()).collect())
        } else {
            None
        }
    }
}

impl<'a> ResolverDepthExt for DependencyResolver<'a> {
    fn depth(&self) -> isize {
        self.depth
    }

    fn set_depth(&mut self, depth: isize) {
        self.depth = depth;
    }
}
