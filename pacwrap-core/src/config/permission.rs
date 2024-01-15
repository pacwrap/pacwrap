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

use crate::exec::args::ExecutionArgs;

use dyn_clone::{clone_trait_object, DynClone};

mod dev;
mod display;
mod env;
mod gpu;
mod net;
pub mod none;
mod pipewire;
mod pulseaudio;

pub enum Condition {
    Success,
    SuccessWarn(String),
    Nothing,
}

#[derive(Debug, Clone)]
pub enum PermError {
    Fail(String),
    Warn(String),
}

#[typetag::serde(tag = "module")]
pub trait Permission: DynClone {
    fn check(&self) -> Result<Option<Condition>, PermError>;
    fn register(&self, args: &mut ExecutionArgs);
    fn module(&self) -> &'static str;
}

impl Display for PermError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Fail(error) => write!(fmter, "{}", error),
            Self::Warn(error) => write!(fmter, "{}", error),
        }
    }
}

clone_trait_object!(Permission);
