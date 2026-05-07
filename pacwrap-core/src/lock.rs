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

use std::{
    fs::{File, remove_file},
    os::unix::fs::MetadataExt,
    path::Path,
};

use thiserror::Error as ThisError;

use crate::{ErrorTrait, PathContext, Result, constants::LOCK_FILE, impl_error};

#[derive(ThisError, Debug)]
pub enum LockError {
    #[error("Lock file is present: '{0}'")]
    Locked(&'static str),
    #[error("Lock not acquired.")]
    NotAcquired,
}

impl_error!(LockError);

pub struct Lock {
    lock: &'static str,
    time: i64,
}

impl Default for Lock {
    fn default() -> Self {
        Self::new()
    }
}

impl Lock {
    pub fn new() -> Self {
        Self {
            lock: *LOCK_FILE,
            time: 0,
        }
    }

    pub fn lock(mut self) -> Result<Self> {
        if self.exists() {
            Err(LockError::Locked(self.lock))?
        }

        File::create(self.lock).context_path(self.lock)?;
        self.time = Path::new(self.lock).metadata().context_path(self.lock)?.ctime();
        Ok(self)
    }

    pub fn assert(&self) -> Result<()> {
        if !self.exists() || Path::new(self.lock).metadata().context_path(self.lock)?.ctime() != self.time {
            Err(LockError::NotAcquired)?
        }

        Ok(())
    }

    pub fn unlock(&self) -> Result<()> {
        Ok(remove_file(self.lock).context_path(self.lock)?)
    }

    pub fn exists(&self) -> bool {
        Path::new(self.lock).exists()
    }
}
