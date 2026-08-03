/*
 * pacwrap
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{File, copy, remove_file},
    io::copy as copy_io,
    process::Command,
};

use pacwrap_core::{
    PathContext,
    Result,
    constants::{ARROW_CYAN, ARROW_GREEN, CONFIG_DIR, DATA_DIR, EDITOR, HOME},
    lock::Lock,
    utils::{Arguments, arguments::Operand},
};
use rand::distr::{Alphanumeric, SampleString};
use sha2::{Digest, Sha256};

pub enum EditKind {
    View,
    Edit,
}

#[derive(Clone, Copy)]
enum FileType<'a> {
    ContainerConfig(&'a str),
    DesktopFile(&'a str),
    Config,
    LogFile,
    Repo,
}

impl<'a> FileType<'a> {
    fn from(str: &'a str) -> Option<FileType<'a>> {
        match str {
            "log" => Some(FileType::LogFile),
            "repo" => Some(FileType::Repo),
            "config" => Some(FileType::Config),
            _ => None,
        }
    }

    fn ext(&self) -> &'static str {
        match self {
            Self::LogFile => ".log",
            Self::ContainerConfig(_) | Self::Config => ".yml",
            Self::DesktopFile(_) => ".desktop",
            Self::Repo => ".conf",
        }
    }

    fn can_edit(&self, edit_type: EditKind) -> EditKind {
        if matches!(self, Self::LogFile) {
            EditKind::View
        } else {
            edit_type
        }
    }
}

impl Display for FileType<'_> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        match self {
            FileType::LogFile => write!(fmt, "{}/pacwrap.log", *DATA_DIR),
            FileType::ContainerConfig(file) => write!(fmt, "{}/container/{}.yml", *CONFIG_DIR, file),
            FileType::DesktopFile(file) => write!(fmt, "{}/.local/share/applications/{}.pacwrap.desktop", *HOME, file),
            FileType::Config => write!(fmt, "{}/pacwrap.yml", *CONFIG_DIR),
            FileType::Repo => write!(fmt, "{}/repositories.conf", *CONFIG_DIR),
        }
    }
}

pub fn edit(args: &mut Arguments, edit_type: EditKind) -> Result<()> {
    let mut file = None;

    while let Some(arg) = args.next() {
        file = Some(match arg {
            Operand::Short('d') | Operand::Long("desktop") | Operand::Long("desktop-any") => continue,
            Operand::Short('l') | Operand::Long("log") | Operand::Value("log") => FileType::LogFile,
            Operand::Short('r') | Operand::Long("repo") | Operand::Value("repo") => FileType::Repo,
            Operand::Short('c') | Operand::Long("config") | Operand::Value("config") => FileType::Config,
            Operand::ShortPos('c', val) | Operand::LongPos("config", val) => FileType::ContainerConfig(val),
            Operand::ShortPos('d', val) | Operand::LongPos("desktop", val) => FileType::DesktopFile(val),
            Operand::LongPos("view", arg)
            | Operand::LongPos("edit", arg)
            | Operand::ShortPos('e', arg)
            | Operand::ShortPos('v', arg) => match FileType::from(arg) {
                Some(f) => f,
                None => return args.invalid_operand(),
            },
            _ => return args.invalid_operand(),
        });
    }

    let (file, ext, lock, edit) = &match file {
        Some(file) => {
            let (edit, ext): (EditKind, &str) = (file.can_edit(edit_type), file.ext());
            let lock = if let (FileType::ContainerConfig(_), &EditKind::Edit) = (file, &edit) {
                Some(Lock::new().lock()?)
            } else {
                None
            };
            let file = file.to_string();

            (file, ext, lock, edit)
        }
        None => return args.invalid_operand(),
    };
    let result = edit_file(file, ext, edit, lock.as_ref());

    if let Some(lock) = lock {
        lock.unlock()?;
    }

    result
}

pub fn edit_file(file: &str, ext: &str, edit_type: &EditKind, lock: Option<&Lock>) -> Result<()> {
    let prs = Alphanumeric.sample_string(&mut rand::rng(), 10);
    let temporary_file = &format!("/tmp/tmp.{}{}", prs, ext);
    let edit = matches!(edit_type, EditKind::Edit);

    copy(file, temporary_file).context_path(file)?;
    Command::new(*EDITOR).arg(temporary_file).spawn().context_path(*EDITOR)?;

    if matches!(edit_type, EditKind::Edit) && hash_file(file)? != hash_file(temporary_file)? {
        if let Some(lock) = lock {
            lock.assert()?;
        }

        copy(temporary_file, file).context_path(file)?;
        eprintln!("{} Changes written to file.", *ARROW_GREEN);
    } else if edit {
        eprintln!("{} No changes made.", *ARROW_CYAN);
    }

    Ok(remove_file(temporary_file)?)
}

fn hash_file(file_path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(file_path).context_path(file_path)?;
    let mut hasher = Sha256::new();

    copy_io(&mut file, &mut hasher).context_path(file_path)?;
    Ok(hasher.finalize().to_vec())
}
