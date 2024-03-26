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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{remove_file, File},
    os::unix::fs::MetadataExt,
    path::Path,
};

use crate::{constants::LOCK_FILE, err, impl_error, Error, ErrorGeneric, ErrorTrait, Result};

#[derive(Debug)]
pub enum LockError {
    Locked(&'static str),
    NotAcquired,
}

impl Display for LockError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Locked(lock) => write!(fmter, "Lock file is present: '{}'", lock),
            Self::NotAcquired => write!(fmter, "Lock not acquired."),
        }?;

        if let Self::Locked(_) = self {
            write!(fmter, "\nTry 'pacwrap -h' for more information on valid operational parameters.")?
        }

        Ok(())
    }
}

impl_error!(LockError);

pub struct Lock {
    lock: &'static str,
    time: i64,
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
            err!(LockError::Locked(self.lock))?
        }

        File::create(self.lock).prepend(|| format!("Failed to create lock file '{}'", self.lock))?;
        self.time = Path::new(self.lock)
            .metadata()
            .prepend(|| format!("Failed to acquire metadata on lock file '{}'", self.lock))?
            .ctime();
        Ok(self)
    }

    pub fn assert(&self) -> Result<()> {
        if !self.exists()
            || Path::new(self.lock)
                .metadata()
                .prepend(|| format!("Failed to acquire metadata on lock file '{}'", self.lock))?
                .ctime()
                != self.time
        {
            err!(LockError::NotAcquired)?
        }

        Ok(())
    }

    pub fn unlock(&self) -> Result<()> {
        Ok(remove_file(self.lock).prepend(|| format!("Failed to remove lock file '{}'", self.lock))?)
    }

    pub fn exists(&self) -> bool {
        Path::new(self.lock).exists()
    }
}
