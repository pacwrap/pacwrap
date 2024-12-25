/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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

use regex::Regex;
use std::{
    fs::{read_dir, remove_file, File},
    io::{Read, Write},
};

use pacwrap_core::{
    config::provide_handle,
    constants::{ARROW_GREEN, HOME},
    err,
    exec::path::resolve_path,
    utils::{arguments::Operand, table::Table, Arguments},
    Error,
    ErrorGeneric,
    ErrorKind,
    Result,
};

const GLOBAL_APP_DIR: &str = "/usr/share/applications";
const LOCAL_APP_DIR: &str = "/.local/share/applications";

pub fn file(args: &mut Arguments) -> Result<()> {
    match args.next().unwrap_or_default() {
        Operand::Short('l') | Operand::Long("list") | Operand::Value("ls") => list_desktop_entries(args),
        Operand::Short('r') | Operand::Long("remove") | Operand::Value("rm") => remove_desktop_entry(args),
        Operand::Short('c') | Operand::Long("create") | Operand::Value("create") => create_desktop_entry(args),
        _ => args.invalid_operand(),
    }
}

fn list_desktop_entries(args: &mut Arguments) -> Result<()> {
    let table_header = vec!["Desktop Entries:"];
    let mut table = Table::new().header(&table_header);
    let (local, app_dir) = &match args.target() {
        Ok(instance) => (false, format!("{}{GLOBAL_APP_DIR}", provide_handle(instance)?.vars().root())),
        Err(_) => (true, format!("{}{LOCAL_APP_DIR}", *HOME)),
    };
    let dir = read_dir(app_dir).prepend_io(|| app_dir)?;

    for entry in dir {
        if let Some(file) = entry.prepend(|| format!("Failure acquiring entry in '{app_dir}'"))?.file_name().to_str() {
            if *local && !file.contains("pacwrap") && !file.ends_with(".desktop") {
                continue;
            }

            table.insert(vec![file.to_string()]);
        }
    }

    print!("{}", table.build()?);
    Ok(())
}

fn create_desktop_entry(args: &mut Arguments) -> Result<()> {
    let target = args.target()?;
    let handle = provide_handle(target)?;
    let app_dir = &format!("{}{GLOBAL_APP_DIR}", handle.vars().root());
    let dir = read_dir(app_dir).prepend_io(|| app_dir)?;
    let name = &match args.next().unwrap_or_default() {
        Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) => val,
        _ => return args.invalid_operand(),
    };
    let mut file_name: Option<String> = None;

    for entry in dir {
        if let Some(file) = entry.prepend(|| format!("Failure acquiring entry in '{app_dir}'"))?.file_name().to_str() {
            if !file.ends_with(".desktop") {
                continue;
            }

            if file.starts_with(name) || file.split_at(file.len() - 8).0.ends_with(name) {
                file_name = Some(file.into());
                break;
            }
        }
    }

    let mut contents = String::new();
    let file_name = &match file_name {
        Some(file) => file,
        None => return err!(ErrorKind::Message("Desktop file not found."))?,
    };
    let desktop_file = &resolve_path(handle.vars().root(), GLOBAL_APP_DIR, "file_name")?;

    File::open(desktop_file)
        .prepend_io(|| desktop_file.to_string_lossy())?
        .read_to_string(&mut contents)
        .prepend_io(|| desktop_file.to_string_lossy())?;
    contents = Regex::new("Exec=*")?
        .replace_all(&contents, format!("Exec=pacwrap run {} ", target))
        .to_string();
    contents = Regex::new("TryExec=(.*)")?.replace_all(&contents, "TryExec=pacwrap").to_string();

    let desktop_file = &format!("{}{LOCAL_APP_DIR}/pacwrap.{}", *HOME, file_name);
    let mut output = File::create(desktop_file).prepend_io(|| desktop_file)?;

    write!(output, "{}", contents).prepend_io(|| desktop_file)?;
    eprintln!("{} Created '{}'.", *ARROW_GREEN, file_name);
    Ok(())
}

fn remove_desktop_entry(args: &mut Arguments) -> Result<()> {
    let app_dir = &format!("{}{LOCAL_APP_DIR}", *HOME);
    let dir = read_dir(app_dir).prepend_io(|| app_dir)?;
    let name = &match args.next().unwrap_or_default() {
        Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) => val,
        _ => return args.invalid_operand(),
    };
    let mut file_name: Option<String> = None;

    for entry in dir {
        if let Some(file) = entry.prepend(|| format!("Failure acquiring entry in '{app_dir}'"))?.file_name().to_str() {
            if !file.contains("pacwrap") || !file.ends_with(".desktop") {
                continue;
            }

            if file.split_at(8).1.starts_with(name) || file.split_at(file.len() - 8).0.ends_with(name) {
                file_name = Some(file.into());
                break;
            }
        }
    }

    let file_name = &match file_name {
        Some(file) => file,
        None => return err!(ErrorKind::Message("Desktop file not found."))?,
    };
    let desktop_file = &format!("{}{LOCAL_APP_DIR}/{}", *HOME, file_name);

    remove_file(desktop_file).prepend_io(|| desktop_file)?;
    eprintln!("{} Removed '{file_name}'.", *ARROW_GREEN);
    Ok(())
}
