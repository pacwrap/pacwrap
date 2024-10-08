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

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::{
    config::{load_config, save},
    constants::CONFIG_FILE,
    sync::event::summary::SummaryKind,
    Result,
};

static CONFIG: OnceLock<Global> = OnceLock::new();

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
    #[serde(default = "SummaryKind::default")]
    summary: SummaryKind,
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
    #[serde(default)]
    disable_sandbox: bool,
}

impl Configuration {
    fn new() -> Self {
        Self {
            summary: SummaryKind::Basic,
            logging: Verbosity::Basic,
            progress: Progress::new(),
        }
    }

    pub fn progress(&self) -> (&ProgressKind, &ProgressKind) {
        (&self.progress.transact, &self.progress.download)
    }

    pub fn logging(&self) -> &Verbosity {
        &self.logging
    }

    pub fn summary(&self) -> &SummaryKind {
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
            sig_level: sig_level(),
            sig_level_local: sig_level_opt(),
            parallel_downloads: parallel_downloads(),
            check_space: true,
            download_timeout: true,
            disable_sandbox: false,
        }
    }

    pub fn sig_level(&self) -> Vec<String> {
        self.sig_level.split(" ").map(|a| a.into()).collect()
    }

    pub fn download_timeout(&self) -> bool {
        !self.download_timeout
    }

    pub fn parallel_downloads(&self) -> u32 {
        self.parallel_downloads
    }

    pub fn check_space(&self) -> bool {
        self.check_space
    }

    pub fn disable_sandbox(&self) -> bool {
        self.disable_sandbox
    }

    pub fn held(&self) -> Vec<&str> {
        self.hold_pkg.iter().map(|a| a.as_ref()).collect()
    }

    pub fn ignored(&self) -> Vec<&str> {
        self.ignore_pkg.iter().map(|a| a.as_ref()).collect()
    }
}

impl Default for Global {
    fn default() -> Self {
        Self::new()
    }
}

impl Global {
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
        save(&self, &CONFIG_FILE)
    }
}

pub fn global() -> Result<&'static Global> {
    Ok(match CONFIG.get() {
        Some(f) => f,
        None => {
            let cfg = match load_config() {
                Ok(config) => Ok(config),
                Err(error) => error.fatal(),
            }?;

            CONFIG.get_or_init(|| cfg)
        }
    })
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
