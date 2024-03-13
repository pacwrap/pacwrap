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

use std::{
    fmt::{Display, Formatter},
    fs::{copy, remove_file, File},
    io::{copy as copy_io, Read},
    process::Command,
};

use pacwrap_core::{
    constants::{ARROW_CYAN, ARROW_GREEN, CONFIG_DIR, DATA_DIR, EDITOR, HOME},
    exec::utils::handle_process,
    utils::{arguments::Operand, Arguments},
    ErrorGeneric,
    Result,
};
use sha2::{Digest, Sha256};

enum FileType<'a> {
    ContainerConfig(&'a str),
    DesktopFile(&'a str),
    Config,
    LogFile,
    Repo,
}

impl<'a> Display for FileType<'a> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            FileType::LogFile => write!(fmt, "{}/pacwrap.log", *DATA_DIR),
            FileType::ContainerConfig(file) => write!(fmt, "{}/container/{}.yml", *CONFIG_DIR, file),
            FileType::DesktopFile(file) => write!(fmt, "{}/.local/share/applications/pacwrap.{}.desktop", *HOME, file),
            FileType::Config => write!(fmt, "{}/pacwrap.yml", *CONFIG_DIR),
            FileType::Repo => write!(fmt, "{}/repositories.conf", *CONFIG_DIR),
        }
    }
}

pub fn edit_file(args: &mut Arguments, edit: bool) -> Result<()> {
    let mut file = None;

    while let Some(arg) = args.next() {
        file = Some(match arg {
            Operand::Short('d') | Operand::Long("desktop") => continue,
            Operand::Short('l') | Operand::Long("log") | Operand::Value("log") => FileType::LogFile,
            Operand::Short('r') | Operand::Long("repo") | Operand::Value("repo") => FileType::Repo,
            Operand::Short('c') | Operand::Long("config") | Operand::Value("config") => FileType::Config,
            Operand::ShortPos('d', val) | Operand::LongPos("desktop", val) => FileType::DesktopFile(val),
            Operand::ShortPos('c', val) | Operand::LongPos("config", val) => FileType::ContainerConfig(val),
            _ => return args.invalid_operand(),
        });
    }

    let temporary_file = &format!("/tmp/tmp.{}", random_string(10)?);
    let file = &match file {
        Some(file) => file.to_string(),
        None => return args.invalid_operand(),
    };

    copy(file, temporary_file).prepend_io(|| file.into())?;
    handle_process(*EDITOR, Command::new(*EDITOR).arg(temporary_file).spawn())?;

    if edit && hash_file(file)? != hash_file(temporary_file)? {
        copy(temporary_file, file).prepend_io(|| temporary_file.into())?;
        eprintln!("{} Changes written to file.", *ARROW_GREEN);
    } else {
        eprintln!("{} No changes made.", *ARROW_CYAN);
    }

    remove_file(temporary_file).prepend_io(|| temporary_file.into())
}

fn hash_file(file_path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(&file_path).prepend_io(|| file_path.into())?;
    let mut hasher = Sha256::new();

    copy_io(&mut file, &mut hasher).prepend_io(|| file_path.into())?;
    Ok(hasher.finalize().to_vec())
}

fn random_string(len: usize) -> Result<String> {
    let mut urand = File::open("/dev/urandom").prepend_io(|| "/dev/urandom".into())?;
    let mut vec: Vec<u8> = Vec::new();

    vec.reserve_exact(len);

    while vec.len() < len {
        let mut buffer = [0; 1];

        urand.read_exact(&mut buffer).prepend_io(|| "/dev/urandom".into())?;

        if buffer[0] > 64 && buffer[0] < 91 || buffer[0] > 96 && buffer[0] < 122 || buffer[0] > 48 && buffer[0] < 58 {
            vec.push(buffer[0]);
        }
    }

    Ok(String::from_utf8_lossy(&vec).to_string())
}
