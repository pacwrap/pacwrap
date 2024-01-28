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

use std::{fs::File, io::Write, path::Path};

use crate::{
    config::global::CONFIG,
    constants::{CACHE_DIR, CONFIG_DIR, DATA_DIR},
    err,
    Error,
    ErrorKind,
    Result,
};

static REPO_CONF_DEFAULT: &'static str = r###"## See the pacman.conf(5) manpage for information on repository directives.
## All other libalpm-related options therein are ignored.

[options]
Architecture = auto

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist 

[multilib]
Include = /etc/pacman.d/mirrorlist
"###;
static PACWRAP_CONF_DEFAULT: &'static str = r###"## See the pacwrap.yml(2) manpage for more detailed information.
## Documentation is also available at https://git.sapphirus.org/pacwrap/pacwrap/docs/

config:
  logging: Basic
  summary: Basic
  #progress:
    #transact: CondensedForeign
    #download: CondensedForeign
alpm:
  #ignore_pkg:
  #- somepackage
  hold_pkg:
  - pacwrap-base-dist
  - pacman
  - glibc
  sig_level: Required DatabaseOptional
  sig_level_local: Optional
  #parallel_downloads: 5
  #check_space: true
  #download_timeout: true"###;

pub struct DirectoryLayout {
    dirs: Vec<&'static str>,
    root: &'static str,
}

impl DirectoryLayout {
    fn instantiate(self) -> Result<()> {
        for dir in self.dirs {
            let path: &str = &format!("{}{}", self.root, dir);

            if Path::new(path).exists() {
                continue;
            }

            if let Err(error) = std::fs::create_dir_all(path) {
                err!(ErrorKind::IOError(path.into(), error.kind()))?
            }
        }

        Ok(())
    }
}

fn data_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!["/root", "/home", "/state", "/pacman/sync"],
        root: *DATA_DIR,
    }
}

fn cache_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!["/pkg"],
        root: *CACHE_DIR,
    }
}

fn config_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!["/instance"],
        root: *CONFIG_DIR,
    }
}

fn write_to_file(location: &str, contents: &str) -> Result<()> {
    if Path::new(&location).exists() {
        return Ok(());
    }

    let mut f = match File::create(&location) {
        Ok(f) => f,
        Err(error) => err!(ErrorKind::IOError(location.into(), error.kind()))?,
    };

    if let Err(error) = write!(f, "{contents}") {
        err!(ErrorKind::IOError(location.into(), error.kind()))?
    }

    Ok(())
}

pub fn init() -> Result<()> {
    config_layout().instantiate()?;
    data_layout().instantiate()?;
    cache_layout().instantiate()?;
    write_to_file(&format!("{}/repositories.conf", *CONFIG_DIR), REPO_CONF_DEFAULT)?;
    write_to_file(&format!("{}/pacwrap.yml", *CONFIG_DIR), PACWRAP_CONF_DEFAULT)?;

    let _ = *CONFIG;

    Ok(())
}
