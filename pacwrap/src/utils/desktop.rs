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
    utils::{arguments::Operand, table::Table, Arguments},
    Error,
    ErrorGeneric,
    ErrorKind,
    Result,
};

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
        Ok(instance) => (false, format!("{}/usr/share/applications", provide_handle(instance)?.vars().root())),
        Err(_) => (true, format!("{}/.local/share/applications", *HOME)),
    };
    let dir = read_dir(app_dir).prepend_io(|| app_dir.into())?;

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
    let app_dir = &format!("{}/usr/share/applications", provide_handle(target)?.vars().root());
    let dir = read_dir(app_dir).prepend_io(|| app_dir.into())?;
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

    let file_name = &match file_name {
        Some(file) => file,
        None => return err!(ErrorKind::Message("Desktop file not found."))?,
    };
    let desktop_file = &format!("{}/{}", app_dir, file_name);
    let mut contents = String::new();

    File::open(desktop_file)
        .prepend_io(|| desktop_file.into())?
        .read_to_string(&mut contents)
        .prepend_io(|| desktop_file.into())?;
    contents = Regex::new("Exec=*")
        .unwrap()
        .replace_all(&contents, format!("Exec=pacwrap run {} ", target))
        .to_string();

    let desktop_file = &format!("{}/.local/share/applications/pacwrap.{}", *HOME, file_name);
    let mut output = File::create(desktop_file).prepend_io(|| desktop_file.into())?;

    write!(output, "{}", contents).prepend_io(|| desktop_file.into())?;
    eprintln!("{} Created '{}'.", *ARROW_GREEN, file_name);
    Ok(())
}

fn remove_desktop_entry(args: &mut Arguments) -> Result<()> {
    let app_dir = &format!("{}/.local/share/applications", *HOME);
    let dir = read_dir(app_dir).prepend_io(|| app_dir.into())?;
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
    let desktop_file = &format!("{}/.local/share/applications/{}", *HOME, file_name);

    remove_file(desktop_file).prepend_io(|| desktop_file.into())?;
    eprintln!("{} Removed '{file_name}'.", *ARROW_GREEN);
    Ok(())
}
