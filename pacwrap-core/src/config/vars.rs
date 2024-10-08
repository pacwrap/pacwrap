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
    env::var,
    fmt::{Debug, Formatter},
};

use crate::constants::{CACHE_DIR, CONFIG_DIR, DATA_DIR};

#[derive(Clone)]
pub struct ContainerVariables {
    home: String,
    root: String,
    user: String,
    config: String,
    instance: String,
    home_mount: String,
    pacman_cache: String,
    pacman_gnupg: String,
}

impl ContainerVariables {
    pub fn new(ins: &str) -> Self {
        Self {
            home: var("PACWRAP_HOME").unwrap_or(format!("{}/home/{ins}", *DATA_DIR)),
            root: var("PACWRAP_ROOT").unwrap_or(format!("{}/root/{ins}", *DATA_DIR)),
            config: format!("{}/container/{ins}.yml", *CONFIG_DIR),
            pacman_gnupg: format!("{}/pacman/gnupg", *DATA_DIR),
            pacman_cache: format!("{}/pkg", *CACHE_DIR),
            home_mount: format!("/home/{ins}"),
            user: ins.into(),
            instance: ins.into(),
        }
    }

    pub fn config(mut self, path: &str) -> Self {
        self.config = path.into();
        self
    }

    pub fn pacman_cache(&self) -> &str {
        &self.pacman_cache
    }

    pub fn pacman_gnupg(&self) -> &str {
        &self.pacman_gnupg
    }

    pub fn config_path(&self) -> &str {
        &self.config
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn home(&self) -> &str {
        &self.home
    }

    pub fn home_mount(&self) -> &str {
        &self.home_mount
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }
}

impl Debug for ContainerVariables {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "Instance:            {}", self.instance)?;
        writeln!(fmter, "Instance User:       {}", self.user)?;
        writeln!(fmter, "Instance Config:     {}", self.config)?;
        writeln!(fmter, "Instance Root:       {}", self.root)?;
        writeln!(fmter, "Instance Home:       {} -> {}", self.home, self.home_mount)
    }
}
