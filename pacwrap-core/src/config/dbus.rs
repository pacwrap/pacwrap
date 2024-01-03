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

use crate::exec::args::ExecutionArgs;

use dyn_clone::{DynClone, clone_trait_object};

mod socket;
mod appindicator;
mod xdg_portal;

#[typetag::serde(tag = "permission")]
pub trait Dbus: DynClone {
    fn register(&self, args: &mut ExecutionArgs);
}

clone_trait_object!(Dbus);
