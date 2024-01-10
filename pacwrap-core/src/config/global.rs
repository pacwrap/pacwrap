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

use std::process::exit;

use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::{Result, config::{config, save}, constants::CONFIG_FILE};

lazy_static! {
    pub static ref CONFIG: Global = Global::load();
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Verbosity {
    None,
    Basic,
    Verbose,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ProgressKind {
    Simple,
    Condensed,
    CondensedForeign,
    CondensedLocal,
    Verbose,
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Verbose
    }
}

impl Default for ProgressKind {
    fn default() -> Self {
        Self::CondensedForeign
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Global {
    #[serde(default = "Configuration::new")]  
    config: Configuration,
    #[serde(default = "AlpmConfiguration::new")]  
    alpm: AlpmConfiguration,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Progress {
    #[serde(default = "ProgressKind::default")] 
    transact: ProgressKind,
     #[serde(default = "ProgressKind::default")] 
    download: ProgressKind,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Configuration {
    #[serde(default = "Verbosity::default")] 
    summary: Verbosity,
    #[serde(default = "Verbosity::default")] 
    logging: Verbosity,
    #[serde(default = "Progress::new")] 
    progress: Progress,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AlpmConfiguration {
    #[serde(default = "ignore_pkg")]
    ignore_pkg: Vec<String>,
    #[serde(default = "hold_pkg")]
    hold_pkg: Vec<String>,
    #[serde(default = "sig_level")] 
    sig_level: String,
    #[serde(default = "sig_level_opt")]  
    sig_level_local: String,
    #[serde(default = "parallel_downloads")] 
    parallel_downloads: u32,
    #[serde(default = "default_true")]  
    check_space: bool,
    #[serde(default = "default_true")]  
    download_timeout: bool,
}

impl Configuration {
    fn new() -> Self {
        Self {
            summary: Verbosity::Basic,
            logging: Verbosity::Basic,
            progress: Progress::new(),
        }
    }

    pub fn progress(&self) -> (&ProgressKind, &ProgressKind) {
        (&self.progress.transact, &self.progress.download)
    }

    pub fn logging(&self) -> &Verbosity {
        &self.summary
    }

    pub fn summary(&self) -> &Verbosity {
        &self.summary
    }
}

impl Progress {
    fn new() -> Self {
        Self {
            transact: ProgressKind::CondensedForeign,
            download: ProgressKind::CondensedForeign, 
        }
    }
}

impl AlpmConfiguration {
    fn new() -> Self {
        Self {
            ignore_pkg: ignore_pkg(),
            hold_pkg: hold_pkg(),
            sig_level:  sig_level(),
            sig_level_local: sig_level_opt(), 
            parallel_downloads: parallel_downloads(),
            check_space: true,
            download_timeout: true,
        }
    }

    pub fn sig_level(&self) -> Vec<String> {
        self.sig_level.split(" ").map(|a| a.into()).collect()
    }

    pub fn download_timeout(&self) -> bool {
        ! self.download_timeout
    }

    pub fn parallel_downloads(&self) -> u32 {
        self.parallel_downloads
    }

    pub fn check_space(&self) -> bool {
        self.check_space
    }

    pub fn held(&self) -> Vec<&str> {
        self.hold_pkg.iter().map(|a| a.as_ref()).collect()
    }

    pub fn ignored(&self) -> Vec<&str> {
        self.ignore_pkg.iter().map(|a| a.as_ref()).collect()
    }
}

impl Global {
    fn load() -> Self {
        match config() {
            Ok(config) => config,
            Err(error) => exit(error.error()),
        }
    }

    pub fn new() -> Self {
        Self {
            config: Configuration::new(),
            alpm: AlpmConfiguration::new(),
        }
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn alpm(&self) -> &AlpmConfiguration {
        &self.alpm
    }

    pub fn save(&self) -> Result<()> {
        save(&self, &*CONFIG_FILE)
    }
}

fn ignore_pkg() -> Vec<String> {
    vec!["".into()]
}

fn hold_pkg() -> Vec<String> {
    vec!["pacwrap-base-dist".into(), "pacman".into(), "glibc".into()]
}

fn sig_level() -> String {
    "Required DatabaseOptional".into()
}

fn sig_level_opt() -> String {
    "Optional".into() 
}

fn parallel_downloads() -> u32 {
    1
}

fn default_true() -> bool {
    true
}
