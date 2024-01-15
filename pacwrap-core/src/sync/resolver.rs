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

use std::collections::HashSet;

use alpm::{Alpm, Package};

use crate::{
    err,
    sync::{utils::AlpmUtils, SyncError},
    Error,
};

pub struct DependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<Package<'a>>,
    keys: Vec<&'a str>,
    ignored: &'a HashSet<String>,
    handle: &'a Alpm,
    depth: isize,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a HashSet<String>) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            keys: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
        }
    }

    fn check_depth(&mut self) -> Result<(), Error> {
        if self.depth == 50 {
            err!(SyncError::RecursionDepthExceeded(self.depth))?
        }

        self.depth += 1;
        Ok(())
    }

    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Result<(Option<Vec<String>>, Vec<Package<'a>>), Error> {
        let mut synchronize: Vec<&'a str> = Vec::new();

        for pkg in packages {
            if let Some(_) = self.resolved.get(pkg) {
                continue;
            }

            if let Some(_) = self.ignored.get(*pkg) {
                continue;
            }

            if let Some(pkg) = self.handle.get_package(pkg) {
                self.packages.push(pkg);
                self.resolved.insert(pkg.name());
                synchronize.extend(
                    pkg.depends()
                        .iter()
                        .filter_map(|p| match self.handle.get_local_package(p.name()) {
                            None => match self.handle.get_package(p.name()) {
                                Some(dep) => Some(dep.name()),
                                None => None,
                            },
                            Some(_) => None,
                        })
                        .collect::<Vec<&str>>(),
                );

                if self.depth > 0 {
                    self.keys.push(pkg.name().into());
                }
            }
        }

        if synchronize.len() > 0 {
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
            let keys = if self.keys.len() > 0 {
                Some(self.keys.iter().map(|a| (*a).into()).collect())
            } else {
                None
            };

            Ok((keys, self.packages))
        }
    }
}
