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

use std::env::var;

fn dist_filesystem_meta() -> String {
    match var("PACWRAP_DIST_META") {
        Ok(var) => var,
        Err(_) => "/usr/share/pacwrap/filesystem.dat".into(),
    }
}

fn dist_filesystem() -> String {
    match var("PACWRAP_DIST_FS") {
        Ok(var) => var,
        Err(_) => "/usr/share/pacwrap/filesystem.tar.zst".into(),
    }
}

fn dist_config() -> String {
    match var("PACWRAP_DIST_CONF") {
        Ok(var) => var,
        Err(_) => "../../../dist/default/pacwrap.yml".into(),
    }
}

fn dist_repo_config() -> String {
    match var("PACWRAP_DIST_REPO_CONF") {
        Ok(var) => var,
        Err(_) => "../../../dist/default/repositories.conf".into(),
    }
}

fn main() {
    if !cfg!(target_os = "linux") || !cfg!(target_family = "unix") {
        panic!("Unsupported build target. Please refer to the documentation for further information.")
    }

    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_FS");
    println!("cargo:rustc-env=PACWRAP_DIST_FS={}", dist_filesystem());
    println!("cargo:rustc-env=PACWRAP_DIST_META={}", dist_filesystem_meta());
    println!("cargo:rustc-env=PACWRAP_DIST_REPO_CONF={}", dist_repo_config());
    println!("cargo:rustc-env=PACWRAP_DIST_CONF={}", dist_config());
}
