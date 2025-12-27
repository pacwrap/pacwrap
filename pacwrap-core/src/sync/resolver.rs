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

use alpm::{Alpm, Package};

use crate::{
    err,
    sync::{Error, SyncError, transaction::TransactionType},
};

const RECURSION_DEPTH_LIMIT: isize = 50;

pub mod local;
pub mod remote;

pub trait Resolver<'a>: Sized {
    fn new(alpm: &'a Alpm) -> Self;
    fn set_ignored(self, ignorelist: &'a HashSet<String>) -> Self;
    fn enumerate(self, packages: &[&'a str]) -> Result<Self, Error>;
    fn packages(&self) -> &Vec<&'a Package>;
}

pub trait ResolverKeys {
    fn keys(&self) -> Option<Vec<String>>;
}

pub trait ResolverFlags {
    fn set_flags(self, trans_type: &TransactionType) -> Self;
}

trait ResolverDepth {
    fn check_depth(&mut self) -> Result<(), Error>;
}

trait ResolverDepthExt {
    fn depth(&self) -> isize;
    fn set_depth(&mut self, depth: isize);
}

impl<'a, T> ResolverDepth for T
where
    T: Resolver<'a> + ResolverDepthExt,
{
    fn check_depth(&mut self) -> Result<(), Error> {
        if self.depth() == RECURSION_DEPTH_LIMIT {
            err!(SyncError::RecursionDepthExceeded(self.depth()))?
        }

        self.set_depth(self.depth() + 1);
        Ok(())
    }
}
