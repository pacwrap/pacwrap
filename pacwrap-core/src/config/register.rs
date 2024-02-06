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

use crate::{
    config::{
        filesystem::{BindError, Filesystem},
        permission::*,
        Dbus,
        ContainerVariables,
        Permission,
        ConfigError,
    },
    err,
    error::*,
    exec::args::ExecutionArgs,
    utils::print_warning,
};

pub fn register_filesystems(per: &Vec<Box<dyn Filesystem>>, vars: &ContainerVariables, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        match p.check(vars) {
            Ok(_) => p.register(args, vars),
            Err(condition) => match condition {
                BindError::Warn(_) => print_warning(ConfigError::Filesystem(p.module(), condition)),
                BindError::Fail(_) => err!(ConfigError::Filesystem(p.module(), condition))?,
            },
        }
    }

    Ok(())
}

pub fn register_permissions(per: &Vec<Box<dyn Permission>>, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        match p.check() {
            Ok(condition) => match condition {
                Some(b) => {
                    p.register(args);

                    if let Condition::SuccessWarn(warning) = b {
                        print_warning(format!("{}: {} ", p.module(), warning));
                    }
                }
                None => continue,
            },
            Err(condition) => match condition {
                PermError::Warn(_) => print_warning(ConfigError::Permission(p.module(), condition)),
                PermError::Fail(_) => err!(ConfigError::Permission(p.module(), condition))?,
            },
        }
    }

    Ok(())
}

pub fn register_dbus(per: &Vec<Box<dyn Dbus>>, args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        p.register(args);
    }

    Ok(())
}
