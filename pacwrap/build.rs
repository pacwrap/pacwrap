/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
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

use pacwrap_core::sync::schema;
use std::{env::var, path::Path, process::Command};

fn head() -> String {
    match Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
        Ok(output) => String::from_utf8(output.stdout).unwrap_or("N/A".into()),
        Err(_) => "N/A".into(),
    }
}

fn time(debug: bool) -> String {
    match debug {
        false => match Command::new("git").args(["log", "-1", "--date=format:%d/%m/%Y", "--format=%ad"]).output() {
            Ok(output) => String::from_utf8(output.stdout).unwrap_or("N/A".into()),
            Err(_) => "N/A".into(),
        },
        true => match Command::new("date").args(["+%d/%m/%Y %T"]).output() {
            Ok(output) => String::from_utf8(output.stdout).unwrap_or("N/A".into()),
            Err(_) => "N/A".into(),
        },
    }
}

fn release(debug: bool) -> &'static str {
    match debug {
        true => "DEV",
        false => "RELEASE",
    }
}

fn is_debug() -> bool {
    var("DEBUG").unwrap().parse().unwrap()
}

fn main() {
    if !cfg!(target_os = "linux") || !cfg!(target_family = "unix") {
        panic!("Unsupported build target. Please refer to the build documentation for further information.")
    } else if !Path::new("../dist/").exists() || !Path::new("../dist/tools/").exists() {
        panic!("Distribution directory is missing. Please refer to the build documentation for further information.")
    } else if !Path::new("../dist/filesystem.tar.zst").exists() {
        panic!("Container fileystem schema is missing. Please refer to the build documentation for further information.")
    }

    let debug: bool = is_debug();

    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_META");
    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_FS");
    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_REPO");
    println!("cargo:rustc-env=PACWRAP_BUILDSTAMP={}", head());
    println!("cargo:rustc-env=PACWRAP_BUILDTIME={}", time(debug));
    println!("cargo:rustc-env=PACWRAP_BUILD={}", release(debug));

    schema::serialize_path("../dist/filesystem.tar.zst", "../dist/filesystem.dat");
}
