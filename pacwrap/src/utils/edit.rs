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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{copy, remove_file, File},
    io::copy as copy_io,
    process::Command,
};

use pacwrap_core::{
    constants::{ARROW_CYAN, ARROW_GREEN, CONFIG_DIR, DATA_DIR, EDITOR, HOME},
    exec::utils::handle_process,
    lock::Lock,
    utils::{arguments::Operand, Arguments},
    ErrorGeneric,
    Result,
};
use rand::distributions::{Alphanumeric, DistString};
use sha2::{Digest, Sha256};

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

    fn can_edit(&self, edit: bool) -> bool {
        !matches!(self, Self::LogFile) && edit
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

pub fn edit(args: &mut Arguments, edit: bool) -> Result<()> {
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
            let (edit, ext) = (file.can_edit(edit), file.ext());
            let lock = if let (FileType::ContainerConfig(_), true) = (file, edit) {
                Some(Lock::new().lock()?)
            } else {
                None
            };
            let file = file.to_string();

            (file, ext, lock, edit)
        }
        None => return args.invalid_operand(),
    };
    let result = edit_file(file, ext, lock.as_ref(), *edit);

    if let Some(lock) = lock {
        lock.unlock()?;
    }

    result
}

pub fn edit_file(file: &str, ext: &str, lock: Option<&Lock>, edit: bool) -> Result<()> {
    let prs = Alphanumeric.sample_string(&mut rand::thread_rng(), 10);
    let temporary_file = &format!("/tmp/tmp.{}{}", prs, ext);

    copy(file, temporary_file).prepend_io(|| file)?;
    handle_process(*EDITOR, Command::new(*EDITOR).arg(temporary_file).spawn())?;

    if edit && hash_file(file)? != hash_file(temporary_file)? {
        if let Some(lock) = lock {
            lock.assert()?;
        }

        copy(temporary_file, file).prepend_io(|| temporary_file)?;
        eprintln!("{} Changes written to file.", *ARROW_GREEN);
    } else if edit {
        eprintln!("{} No changes made.", *ARROW_CYAN);
    }

    remove_file(temporary_file).prepend_io(|| temporary_file)
}

fn hash_file(file_path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(file_path).prepend_io(|| file_path)?;
    let mut hasher = Sha256::new();

    copy_io(&mut file, &mut hasher).prepend_io(|| file_path)?;
    Ok(hasher.finalize().to_vec())
}
