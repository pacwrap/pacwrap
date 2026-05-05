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

use std::{
    io::ErrorKind::NotFound,
    path::{Path, PathBuf},
};

use thiserror::Error as ThisError;

use crate::{
    Error,
    ErrorGeneric,
    ErrorTrait,
    Result,
    config::{ContainerHandle, ContainerType::Slice},
    constants::{BOLD, RESET},
    err,
    exec::{DIST_IMG, ExecutionError},
    impl_error,
};

#[derive(ThisError, Debug, Clone)]
pub enum PathError {
    #[error("'{0}': {bold}PATH{reset} variable must be absolute", bold=*BOLD, reset=*RESET)]
    UnabsolutePath(String),
    #[error("'{0}': Executable path must be absolute.")]
    UnabsoluteExec(String),
    #[error("'{0}': No such file or directory in container.")]
    PathUnresolvable(String),
}

impl_error!(PathError);

pub fn check_path(ins: &ContainerHandle, args: &[&str], path: Vec<&str>) -> Result<()> {
    if let (Slice, true) = (ins.metadata().container_type(), !args.is_empty()) {
        if resolve_path(*DIST_IMG, "/bin", args[0]).is_ok() {
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
                if resolve_path(ins.vars().root(), dir, args[0]).is_ok() {
                    return Ok(());
                },
            Err(error) => err!(ExecutionError::InvalidPathVar(dir.into(), error.kind()))?,
        }
    }

    err!(ExecutionError::ExecutableUnavailable(args[0].into()))?
}

pub fn resolve_path(root: &str, dir: &str, file: &str) -> Result<PathBuf> {
    if file.contains("..") {
        err!(PathError::UnabsoluteExec(file.into()))?
    } else if dir.contains("..") {
        err!(PathError::UnabsolutePath(file.into()))?
    }

    let path = format!("{}{}/{}", root, dir, file);
    let path = obtain_path(Path::new(&path)).prepend_io(|| file)?;
    let path_direct = format!("{}/{}", root, file);
    let path_direct = obtain_path(Path::new(&path_direct)).prepend_io(|| file)?;

    if let Ok(path) = path.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return resolve_path(root, dir, path);
        }
    } else if let Ok(path) = path_direct.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return resolve_path(root, dir, path);
        }
    }

    if path_direct.exists() {
        Ok(path_direct)
    } else if path.exists() {
        Ok(path)
    } else {
        err!(PathError::PathUnresolvable(format!("{dir}/{file}")))
    }
}

fn obtain_path(path: &Path) -> Result<PathBuf> {
    match Path::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(err) => match err.kind() {
            NotFound => Ok(path.to_path_buf()),
            _ => Err(err)?,
        },
    }
}
