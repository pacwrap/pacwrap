/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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

use std::path::{Path, PathBuf};

use crate::{
    config::{ContainerHandle, ContainerType::Slice},
    err,
    exec::{ExecutionError, DIST_IMG},
    Error,
    ErrorKind,
    Result,
};

pub fn check_path(ins: &ContainerHandle, args: &[&str], path: Vec<&str>) -> Result<()> {
    if let (Slice, true) = (ins.metadata().container_type(), !args.is_empty()) {
        if dest_exists(*DIST_IMG, "/bin", args[0])? {
            return Ok(());
        }

        err!(ExecutionError::ExecutableUnavailable(args[0].into()))?
    }

    if args.is_empty() {
        err!(ExecutionError::RuntimeArguments)?
    }

    for dir in path {
        match Path::new(&format!("{}/{}", ins.vars().root(), dir)).try_exists() {
            Ok(_) =>
                if dest_exists(ins.vars().root(), dir, args[0])? {
                    return Ok(());
                },
            Err(error) => err!(ExecutionError::InvalidPathVar(dir.into(), error.kind()))?,
        }
    }

    err!(ExecutionError::ExecutableUnavailable(args[0].into()))?
}

fn dest_exists(root: &str, dir: &str, exec: &str) -> Result<bool> {
    if exec.contains("..") {
        err!(ExecutionError::UnabsoluteExec(exec.into()))?
    } else if dir.contains("..") {
        err!(ExecutionError::UnabsolutePath(exec.into()))?
    }

    let path = format!("{}{}/{}", root, dir, exec);
    let path = obtain_path(Path::new(&path), exec)?;
    let path_direct = format!("{}/{}", root, exec);
    let path_direct = obtain_path(Path::new(&path_direct), exec)?;

    if path.is_dir() | path_direct.is_dir() {
        err!(ExecutionError::DirectoryNotExecutable(exec.into()))?
    } else if let Ok(path) = path.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return dest_exists(root, dir, path);
        }
    } else if let Ok(path) = path_direct.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return dest_exists(root, dir, path);
        }
    }

    Ok(path.exists() | path_direct.exists())
}

fn obtain_path(path: &Path, exec: &str) -> Result<PathBuf> {
    match Path::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
            _ => err!(ErrorKind::IOError(exec.into(), err.kind())),
        },
    }
}
