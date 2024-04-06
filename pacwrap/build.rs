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
use std::{env::var, os::unix::fs::MetadataExt, path::Path, process::Command};

fn head() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .and_then(|output| Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 value")))
        .unwrap_or(String::new())
}

fn tag() -> String {
    Command::new("git")
        .args(["tag", "--points-at"])
        .output()
        .and_then(|output| Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 value")))
        .unwrap_or(String::new())
}

fn time(debug: bool) -> String {
    match debug {
        false => Command::new("git")
            .args(["log", "-1", "--date=format:%d/%m/%Y", "--format=%cd"])
            .output()
            .and_then(|output| Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 value")))
            .and_then(|date| Ok(date.is_empty().then(|| mtime()).unwrap_or(date)))
            .unwrap_or(mtime()),
        true => Command::new("date")
            .args(["+%d/%m/%Y %T%:z"])
            .output()
            .and_then(|output| Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 value")))
            .expect("'date': executable not found in PATH"),
    }
}

fn mtime() -> String {
    Command::new("date")
        .args(["+%d/%m/%Y", "--utc", "--date"])
        .arg(format!("@{}", Path::new(".").metadata().expect("Metadata expected for src directory").mtime()))
        .output()
        .and_then(|output| Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 value")))
        .expect("'date': executable not found in PATH")
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
    let built = var("PACWRAP_SCHEMA_BUILT").is_ok();

    if !cfg!(target_os = "linux") || !cfg!(target_family = "unix") {
        panic!("Unsupported build target. Please refer to the build documentation for further information.")
    } else if built && (!Path::new("../dist/").exists() || !Path::new("../dist/tools/").exists()) {
        panic!("Distribution directory is missing. Please refer to the build documentation for further information.")
    } else if built && !Path::new("../dist/bin/filesystem.tar.zst").exists() {
        panic!("Container fileystem schema is missing. Please refer to the build documentation for further information.")
    }

    let debug: bool = is_debug();

    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_META");
    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_FS");
    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_REPO");
    println!("cargo:rustc-env=PACWRAP_BUILD={}", release(debug));
    println!("cargo:rustc-env=PACWRAP_BUILDSTAMP={}", time(debug));
    println!("cargo:rustc-env=PACWRAP_BUILDHEAD={}", head());
    println!("cargo:rustc-env=PACWRAP_BUILDTAG={}", tag());

    if built {
        schema::serialize_path("../dist/bin/filesystem.tar.zst", "../dist/bin/filesystem.dat");
    }
}
