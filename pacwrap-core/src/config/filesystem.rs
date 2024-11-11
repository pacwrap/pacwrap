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
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
};

use dyn_clone::{clone_trait_object, DynClone};
use serde::{
    de::{Error as DeError, Visitor},
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};

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

#[derive(Debug, PartialEq, Clone)]
pub enum Permission {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Mount {
    #[serde(skip_serializing_if = "is_default_permission", default = "default_permission")]
    permission: Permission,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    path: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    dest: String,
}

struct PermissionVisitor;

impl Serialize for Permission {
    fn serialize<D: Serializer>(&self, serializer: D) -> StdResult<D::Ok, D::Error> {
        serializer.serialize_str(self.into())
    }
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D: Deserializer<'de>>(serializer: D) -> StdResult<Self, D::Error> {
        serializer.deserialize_str(PermissionVisitor)
    }
}

impl<'de> Visitor<'de> for PermissionVisitor {
    type Value = Permission;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "expected the type 'ro' or 'rw'")
    }

    fn visit_str<E: DeError>(self, str: &str) -> StdResult<Self::Value, E> {
        let str_lc = str.to_lowercase();

        if str_lc != "rw" && str_lc != "ro" {
            Err(E::invalid_type(serde::de::Unexpected::Other(str), &self))?
        }

        Ok(str.into())
    }
}

impl Mount {
    fn dest(per: Permission, path: &str) -> Self {
        Self {
            permission: per,
            path: path.into(),
            dest: path.into(),
        }
    }
}

impl From<&str> for Permission {
    fn from(s: &str) -> Permission {
        match s.to_lowercase().as_str() {
            "rw" => Self::ReadWrite,
            _ => Self::ReadOnly,
        }
    }
}

impl From<&Permission> for &str {
    fn from(val: &Permission) -> &'static str {
        match val {
            Permission::ReadWrite => "rw",
            Permission::ReadOnly => "ro",
        }
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

fn default_permission() -> Permission {
    Permission::ReadOnly
}

fn is_default_permission(var: &Permission) -> bool {
    var == &Permission::ReadOnly
}
