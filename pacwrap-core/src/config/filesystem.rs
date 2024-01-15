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

use std::fmt::{Display, Formatter};

use crate::{config::InsVars, exec::args::ExecutionArgs};

use dyn_clone::{clone_trait_object, DynClone};

mod dir;
pub mod home;
pub mod root;
mod sys;
mod to_home;
mod to_root;

pub enum Condition {
    Success,
    SuccessWarn(String),
    Nothing,
}

#[derive(Debug, Clone)]
pub enum BindError {
    Fail(String),
    Warn(String),
}

impl Display for BindError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Fail(error) => write!(fmter, "{}", error),
            Self::Warn(error) => write!(fmter, "{}", error),
        }
    }
}

#[typetag::serde(tag = "mount")]
pub trait Filesystem: DynClone {
    fn check(&self, vars: &InsVars) -> Result<(), BindError>;
    fn register(&self, args: &mut ExecutionArgs, vars: &InsVars);
    fn module(&self) -> &'static str;
}

clone_trait_object!(Filesystem);

fn default_permission() -> String {
    "ro".into()
}

fn is_default_permission(var: &String) -> bool {
    var == "ro"
}
