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

use std::{
    fmt::{Display, Error as FmtError, Formatter},
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{BufReader, ErrorKind as IOErrorKind, Read, Seek},
    path::Path,
    process::exit,
    result::Result as StdResult,
};

use indexmap::IndexSet;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tar::{Archive, EntryType};
use zstd::Decoder;

use self::SchemaStatus::*;

use crate::{
    config::ContainerHandle,
    constants::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH},
    err,
    impl_error,
    utils::{print_warning, read_le_32},
    Error,
    ErrorKind,
    ErrorTrait,
    Result,
};

const MAGIC_NUMBER: u32 = 659933704;
const ARCHIVE_PATH: &'static str = env!("PACWRAP_DIST_FS");
const SCHEMA_META: &'static str = ".container_schema";

lazy_static! {
    pub static ref SCHEMA_STATE: SchemaState = match deserialize() {
        Ok(s) => s,
        Err(e) => exit(e.error()),
    };
}

#[derive(Debug, Clone)]
pub enum SchemaError {
    CopyFailure(String, IOErrorKind),
    RemovalFailure(String, IOErrorKind),
    DeserializationError(String, String),
}

impl Display for SchemaError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> StdResult<(), FmtError> {
        match self {
            Self::CopyFailure(dir, err) => write!(fmter, "Failed to copy '{dir}': {err}"),
            Self::RemovalFailure(dir, err) => write!(fmter, "Failed to remove '{dir}': {err}"),
            Self::DeserializationError(dir, err) => write!(fmter, "Schema deserialization failure '{dir}': {err}"),
        }
    }
}

impl_error!(SchemaError);

pub enum SchemaStatus {
    UpToDate,
    OutOfDate(Option<SchemaState>),
}

#[derive(Serialize, Deserialize, Eq, Hash, Debug, PartialEq)]
enum NodeType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Serialize, Deserialize)]
pub struct SchemaState {
    magic: u32,
    major: u32,
    minor: u32,
    patch: u32,
    files: IndexSet<SchemaNode>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct SchemaNode {
    node_path: String,
    node_type: NodeType,
}

impl SchemaState {
    fn new() -> Self {
        Self {
            magic: MAGIC_NUMBER,
            major: *VERSION_MAJOR,
            minor: *VERSION_MINOR,
            patch: *VERSION_PATCH,
            files: IndexSet::new(),
        }
    }
}

impl Hash for SchemaNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_path.hash(state)
    }
}

impl From<(String, NodeType)> for SchemaNode {
    fn from((file_path, file_type): (String, NodeType)) -> SchemaNode {
        Self {
            node_path: file_path,
            node_type: file_type,
        }
    }
}

impl From<EntryType> for NodeType {
    fn from(metadata: EntryType) -> Self {
        match metadata {
            EntryType::Regular => Self::File,
            EntryType::Symlink => Self::Symlink,
            EntryType::Directory => Self::Directory,
            _ => Self::Other,
        }
    }
}

pub fn extract(inshandle: &ContainerHandle, old_schema: &Option<SchemaState>) -> Result<()> {
    let meta_path = format!("{}/{}", inshandle.vars().root(), SCHEMA_META);

    if let Some(schema) = old_schema {
        for file in schema
            .files
            .iter()
            .filter(|a| SCHEMA_STATE.files.get(*a).is_none())
            .rev()
            .collect::<IndexSet<&SchemaNode>>()
        {
            let path = format!("{}/{}", inshandle.vars().root(), file.node_path);

            if let Err(error) = match file.node_type {
                NodeType::File => remove_file(path),
                NodeType::Directory => remove_directory(path),
                NodeType::Symlink => remove_symlink(path),
                NodeType::Other => continue,
            } {
                error.warn();
            }
        }
    }

    for entry in access_archive(ARCHIVE_PATH)?.entries().unwrap() {
        let mut entry = match entry {
            Ok(entry) => entry,
            Err(error) => err!(ErrorKind::IOError(ARCHIVE_PATH.into(), error.kind()))?,
        };
        let path = match entry.path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(err) => err!(ErrorKind::IOError(ARCHIVE_PATH.into(), err.kind()))?,
        };
        let dest_path = format!("{}/{}", inshandle.vars().root(), path);

        if let Err(error) = entry.unpack(&dest_path) {
            err!(ErrorKind::IOError(dest_path, error.kind()))?
        }
    }

    if let Err(err) = fs::copy(env!("PACWRAP_DIST_META"), &meta_path) {
        err!(ErrorKind::IOError(meta_path, err.kind()))?
    }

    Ok(())
}

pub fn version(inshandle: &ContainerHandle) -> Result<SchemaStatus> {
    let mut header_buffer = vec![0; 16];
    let schema: &str = &format!("{}/{}", inshandle.vars().root(), SCHEMA_META);
    let mut file = match File::open(&schema) {
        Ok(file) => file,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(OutOfDate(None));
            }

            err!(ErrorKind::IOError(schema.into(), err.kind()))?
        }
    };

    if let Err(error) = file.read_exact(&mut header_buffer) {
        err!(ErrorKind::IOError(schema.into(), error.kind()))?
    }

    let magic = read_le_32(&header_buffer, 0);
    let major: (u32, u32) = (*VERSION_MAJOR, read_le_32(&header_buffer, 4));
    let minor: (u32, u32) = (*VERSION_MINOR, read_le_32(&header_buffer, 8));
    let patch: (u32, u32) = (*VERSION_PATCH, read_le_32(&header_buffer, 12));

    if let Err(error) = file.rewind() {
        err!(ErrorKind::IOError(schema.into(), error.kind()))?
    }

    if magic != MAGIC_NUMBER {
        print_warning(format!("'{}': Magic number mismatch ({MAGIC_NUMBER} != {magic})", schema));
        Ok(OutOfDate(None))
    } else if major.0 != major.1 || minor.0 != minor.1 || patch.0 != patch.1 {
        Ok(OutOfDate(match bincode::deserialize_from::<&File, SchemaState>(&file) {
            Ok(ver) => Some(ver),
            Err(err) => err!(SchemaError::DeserializationError(schema.into(), err.to_string()))?,
        }))
    } else {
        Ok(UpToDate)
    }
}

pub fn serialize_path(from: &str, dest: &str) {
    let mut schema = SchemaState::new();
    let file = File::create(dest).unwrap();

    for entry in access_archive(from).unwrap().entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into();
        let entry_type = entry.header().entry_type().into();

        schema.files.insert((path, entry_type).into());
    }

    bincode::serialize_into(file, &schema).unwrap();
}

fn deserialize() -> Result<SchemaState> {
    let schema = env!("PACWRAP_DIST_META");
    let file = match File::open(schema) {
        Ok(file) => file,
        Err(err) => err!(ErrorKind::IOError(schema.into(), err.kind()))?,
    };

    match bincode::deserialize_from::<&File, SchemaState>(&file) {
        Ok(ver) => Ok(ver),
        Err(err) => err!(SchemaError::DeserializationError(schema.into(), err.to_string())),
    }
}

fn access_archive<'a>(path: &str) -> Result<Archive<Decoder<'a, BufReader<File>>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => err!(ErrorKind::IOError(path.into(), err.kind()))?,
    };
    let file = match Decoder::new(file) {
        Ok(file) => file,
        Err(err) => err!(ErrorKind::IOError(path.into(), err.kind()))?,
    };

    Ok(Archive::new(file))
}

fn remove_file(path: String) -> Result<()> {
    let pacnew_path = format!("{}.pacnew", &path);

    match Path::new(&pacnew_path).exists() {
        false => match fs::remove_file(&path) {
            Ok(_) => Ok(()),
            Err(err) => err!(SchemaError::RemovalFailure(path, err.kind())),
        },
        true => match fs::copy(&pacnew_path, &path) {
            Ok(_) => Ok(()),
            Err(err) => err!(SchemaError::CopyFailure(pacnew_path, err.kind())),
        },
    }
}

fn remove_symlink(path: String) -> Result<()> {
    if let Ok(_) = fs::read_link(&path) {
        if let Err(err) = fs::remove_file(&path) {
            err!(SchemaError::RemovalFailure(path, err.kind()))?
        }
    }

    Ok(())
}

fn remove_directory(path: String) -> Result<()> {
    if is_directory_occupied(&path)? {
        return Ok(());
    }

    match fs::remove_dir(&path) {
        Ok(_) => Ok(()),
        Err(err) => err!(SchemaError::RemovalFailure(path, err.kind())),
    }
}

fn is_directory_occupied(path: &str) -> Result<bool> {
    match fs::read_dir(path) {
        Ok(dir) => Ok(dir.count() > 0),
        Err(err) => err!(ErrorKind::IOError(path.into(), err.kind()))?,
    }
}
