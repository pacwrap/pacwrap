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

use std::fmt::{Display, Formatter, Result as FmtResult};

use dyn_clone::{clone_trait_object, DynClone};
use serde::{Deserialize, Serialize};

use crate::{config::ContainerVariables, exec::args::ExecutionArgs, impl_error, ErrorTrait, Result};

mod dir;
pub mod home;
pub mod root;
mod sys;
mod tmp;
mod to_home;
mod to_root;
mod xdg_home;

#[typetag::serde(tag = "mount")]
pub trait Filesystem: DynClone {
    fn qualify(&self, vars: &ContainerVariables) -> Result<()>;
    fn register(&self, args: &mut ExecutionArgs, vars: &ContainerVariables);
    fn module(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum BindError {
    Fail(String),
    Warn(String),
}

enum Permission {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Mount {
    #[serde(skip_serializing_if = "is_default_permission", default = "default_permission")]
    permission: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    path: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    dest: String,
}

impl Mount {
    fn dest(per: Permission, path: &str) -> Self {
        Self {
            permission: per.into(),
            path: path.into(),
            dest: path.into(),
        }
    }
}

impl From<&str> for Permission {
    fn from(s: &str) -> Permission {
        match s.to_lowercase().as_str() {
            "rw" => Self::ReadWrite,
            _  => Self::ReadOnly,
        }
    }
}

impl From<Permission> for String {
    fn from(val: Permission) -> String {
        match val {
            Permission::ReadWrite => "rw",
            Permission::ReadOnly => "ro",
        }
        .into()
    }
}

impl Display for BindError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Fail(error) => write!(fmter, "{}", error),
            Self::Warn(error) => write!(fmter, "{}", error),
        }
    }
}

impl_error!(BindError);
clone_trait_object!(Filesystem);

fn default_permission() -> String {
    "ro".into()
}

fn is_default_permission(var: &String) -> bool {
    var == "ro"
}
