/*
 * pacwrap
 *
 * Copyright (C) 2023-2025 Xavier Moffett <sapphirus@azorium.net>
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
    cmp::Ordering::{self, *},
    fs::{read_dir, remove_file, File},
    io::{Read, Write},
    result::Result as StdResult,
};

use pacwrap_core::{
    config::{provide_handle, ContainerHandle},
    constants::{ARROW_GREEN, HOME},
    err,
    exec::path::resolve_path,
    utils::{arguments::Operand, table::Table, Arguments},
    Error,
    ErrorGeneric,
    ErrorKind,
    Result,
};

use crate::{
    help::{help, HelpTopic},
    utils::edit::edit_file,
};

const GLOBAL_APP_DIR: &str = "/usr/share/applications";
const LOCAL_APP_DIR: &str = "/.local/share/applications";

struct DesktopEntry<'a> {
    name: String,
    title: Option<String>,
    container: Option<String>,
    handle: Option<&'a ContainerHandle<'a>>,
}

struct DesktopMeta<'a> {
    handle: Option<&'a ContainerHandle<'a>>,
    app_dir: String,
}

impl<'a> DesktopMeta<'a> {
    fn new(hdle: Option<&'a ContainerHandle<'a>>) -> Self {
        DesktopMeta {
            handle: hdle,
            app_dir: match hdle {
                Some(handle) => format!("{}{GLOBAL_APP_DIR}", handle.vars().root()),
                None => format!("{}{LOCAL_APP_DIR}", *HOME),
            },
        }
    }
}

impl From<DesktopEntry<'_>> for Vec<String> {
    fn from(val: DesktopEntry<'_>) -> Vec<String> {
        let container = match val.handle {
            Some(handle) => handle.vars().instance().into(),
            None => val.container.unwrap_or("-".into()),
        };
        let name = val.title.unwrap_or("-".into());
        let file = val.name;

        vec![name, file, container]
    }
}

pub fn file(args: &mut Arguments) -> Result<()> {
    match args.next().unwrap_or_default() {
        Operand::Short('l') | Operand::Long("list") | Operand::Value("list") => list(args),
        Operand::Short('r') | Operand::Long("remove") | Operand::Value("remove") => remove(args),
        Operand::Short('c') | Operand::Long("create") | Operand::Value("create") => create(args),
        Operand::Short('e') | Operand::Long("edit") | Operand::Value("edit") => edit(args),
        Operand::Short('h') | Operand::Long("help") => help(args, &HelpTopic::Utils),
        _ => args.invalid_operand(),
    }
}

fn list(args: &mut Arguments) -> Result<()> {
    let mut table = Table::new().header(&["Application", "Entry", "Container"]);
    let mut handle: Option<ContainerHandle<'_>> = None;
    let mut predicate = "";

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('f') | Operand::Long("find") => continue,
            Operand::ShortPos('f', val) | Operand::LongPos("find", val) => predicate = val,
            Operand::ShortPos('l', val) | Operand::LongPos("list", val) | Operand::Value(val) =>
                handle = Some(provide_handle(val)?),
            _ => args.invalid_operand()?,
        }
    }

    let mut elements = desktop_entries(&DesktopMeta::new(handle.as_ref()), predicate)?;

    elements.sort_by(|a, b| sort_desktop_entry(a, b));
    table.extend(elements.into_iter().map(|e| e.into()).collect());
    print!("{}", table.build()?);
    Ok(())
}

fn map_desktop_entry<'a>(meta: &DesktopMeta<'a>, file: &str) -> Option<DesktopEntry<'a>> {
    if let (None, false) = (&meta.handle, file.contains("pacwrap") && file.contains("desktop")) {
        return None;
    }

    match read_desktop_entry(meta, file) {
        Ok(entry) => Some(entry),
        Err(err) => {
            err.warn();
            None
        }
    }
}

fn sort_desktop_entry<'a>(a: &'a DesktopEntry<'a>, b: &'a DesktopEntry<'a>) -> Ordering {
    let (a, b) = (a.container.as_ref(), b.container.as_ref());

    if let (None, None) = (a, b) {
        Equal
    } else if let (Some(_), None) = (a, b) {
        Less
    } else {
        Greater
    }
}

fn edit(args: &mut Arguments) -> Result<()> {
    let meta = DesktopMeta::new(None);
    let name = &match args.next().unwrap_or_default() {
        Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) => val,
        _ => return args.invalid_operand(),
    };
    let desktop = desktop_entries(&meta, name)?;
    let app_dir = meta.app_dir;

    match desktop.first() {
        Some(entry) => edit_file(&format!("{app_dir}/{}", entry.name), ".desktop", None, false),
        None => err!(ErrorKind::Message("Desktop file not found.")),
    }
}

fn create(args: &mut Arguments) -> Result<()> {
    let target = args.target()?;
    let handle = provide_handle(target)?;
    let meta = DesktopMeta::new(Some(&handle));
    let name = &match args.next().unwrap_or_default() {
        Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) => val,
        _ => return args.invalid_operand(),
    };
    let entries = desktop_entries(&meta, name)?;

    if entries.is_empty() {
        err!(ErrorKind::Message("Desktop file(s) not found."))?;
    }

    for entry in entries {
        create_desktop_entry(&handle, &entry.name, target)?;
    }

    Ok(())
}

fn remove(args: &mut Arguments) -> Result<()> {
    let name = &match args.next().unwrap_or_default() {
        Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) => val,
        _ => return args.invalid_operand(),
    };
    let entries = desktop_entries(&DesktopMeta::new(None), name)?;

    if entries.is_empty() {
        err!(ErrorKind::Message("Desktop file(s) not found."))?
    }

    for entry in entries {
        let file_name = entry.name;
        let desktop_file = &format!("{}{LOCAL_APP_DIR}/{}", *HOME, file_name);

        remove_file(desktop_file).prepend_io(|| desktop_file)?;
        eprintln!("{} Removed '{file_name}'.", *ARROW_GREEN);
    }

    Ok(())
}

fn read_desktop_entry<'a>(meta: &DesktopMeta<'a>, file: &str) -> Result<DesktopEntry<'a>> {
    let mut contents = String::new();
    let mut entry: Option<&str> = None;
    let mut cont: Option<String> = None;
    let mut name: Option<String> = None;
    let app_dir = &meta.app_dir;
    let hdle = meta.handle;
    let path = match hdle {
        Some(handle) => resolve_path(handle.vars().root(), GLOBAL_APP_DIR, file)?,
        None => format!("{app_dir}/{file}").into(),
    };
    let path = path.to_string_lossy();

    File::open(&*path)
        .prepend_io(|| &path)?
        .read_to_string(&mut contents)
        .prepend_io(|| &path)?;

    for line in contents.lines().filter(|f| !f.starts_with("#")) {
        if let (true, Some(start), Some(end)) = (line.starts_with("["), line.find("["), line.find("]")) {
            entry = Some(&line[start + 1 .. end]);
        } else if let Some(entry) = entry {
            let (key, value) = line_slice(line);

            if entry == "pacwrap" && key == "container" {
                cont = Some(value.into());
            } else if entry == "Desktop Entry" && key == "Name" {
                name = Some(value.into());
            }
        }
    }

    Ok(DesktopEntry {
        name: file.into(),
        title: name,
        container: cont,
        handle: hdle,
    })
}

fn line_slice(value: &str) -> (&str, &str) {
    let indices = value.char_indices();
    let mut last: Option<(usize, char)> = None;
    let (mut key_start, mut key_end) = (0, 0);
    let (mut val_start, val_end) = (0, value.len());
    let mut delimiter = false;

    for next in indices {
        if let (Some((_, ' ')), (idx, char)) = (last, next) {
            if char == '=' {
                delimiter = true;
                val_start = idx;
                continue;
            }

            if char != ' ' && char != '\t' {
                if !delimiter {
                    key_start = idx;
                } else {
                    val_start = idx;
                    break;
                }
            }
        }

        if let ((_, ' '), Some((idx, char))) = (next, last) {
            if char != ' ' && char != '\t' {
                if !delimiter {
                    key_end = idx + 1;
                } else {
                    break;
                }
            }
        }

        if let (idx, '=') = next {
            if !delimiter {
                if key_end == 0 {
                    key_end = idx;
                }

                val_start = idx + 1;
                delimiter = true;
            }
        }

        last = Some(next);
    }

    (&value[key_start .. key_end], &value[val_start .. val_end])
}

fn create_desktop_entry(handle: &ContainerHandle, file_name: &str, target: &str) -> Result<()> {
    let desktop_file = &resolve_path(handle.vars().root(), GLOBAL_APP_DIR, file_name)?;
    let mut contents = String::new();

    File::open(desktop_file)
        .prepend_io(|| desktop_file.to_string_lossy())?
        .read_to_string(&mut contents)
        .prepend_io(|| desktop_file.to_string_lossy())?;
    contents = Regex::new("Exec=*")?
        .replace_all(&contents, format!("Exec=pacwrap run {} ", target))
        .to_string();
    contents = Regex::new("TryExec=(.*)")?.replace_all(&contents, "TryExec=pacwrap").to_string();
    contents.push_str(&format!("\n[pacwrap]\ncontainer={target}"));

    let file_name = &file_name[.. file_name.len() - 8];
    let desktop_file = &format!("{}{LOCAL_APP_DIR}/{file_name}.pacwrap.desktop", *HOME);
    let mut output = File::create(desktop_file).prepend_io(|| desktop_file)?;

    write!(output, "{}", contents).prepend_io(|| desktop_file)?;
    eprintln!("{} Created '{file_name}.pacwrap.desktop'.", *ARROW_GREEN);
    Ok(())
}

fn desktop_entries<'a>(meta: &DesktopMeta<'a>, predicate: &str) -> Result<Vec<DesktopEntry<'a>>> {
    Ok(read_dir(&meta.app_dir)
        .prepend_io(|| &meta.app_dir)?
        .filter_map(StdResult::ok)
        .filter_map(|entry| {
            let file = entry.file_name();
            let file = file.to_string_lossy();
            let name = file.split_at(file.len() - 8).0;

            if name.starts_with(predicate) || name.starts_with(predicate) {
                Some(file.to_string())
            } else {
                None
            }
        })
        .filter_map(|file| map_desktop_entry(meta, &file))
        .collect::<Vec<DesktopEntry<'_>>>())
}
