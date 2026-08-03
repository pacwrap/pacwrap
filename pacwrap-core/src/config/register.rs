/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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
        ConfigError,
        ContainerVariables,
        Dbus,
        Permission,
        filesystem::{BindError, Filesystem},
        permission::*,
    },
    eprintln_warn,
    error::*,
    exec::args::ExecutionArgs,
};

pub fn register_filesystems(per: &Vec<Box<dyn Filesystem>>, vars: &ContainerVariables, args: &mut ExecutionArgs) -> Result<()> {
    for filesystem in per {
        match filesystem.qualify(vars) {
            Ok(_) => filesystem.register(args, vars),
            Err(condition) =>
                if let ErrorType::Bind(condition) = condition.error {
                    match condition {
                        BindError::Warn(_) => ConfigError::Filesystem(filesystem.module(), condition).warn(),
                        BindError::Fail(_) => Err(ConfigError::Filesystem(filesystem.module(), condition))?,
                    }
                },
        }
    }

    Ok(())
}

pub fn register_permissions(per: &[Box<dyn Permission>], args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        match p.qualify() {
            Ok(condition) => match condition {
                Some(b) => {
                    p.register(args);

                    if let Condition::SuccessWarn(warning) = b {
                        eprintln_warn!("{}: {} ", p.module(), warning);
                    }
                }
                None => continue,
            },
            Err(condition) => match condition {
                PermError::Warn(_) => ConfigError::Permission(p.module(), condition).warn(),
                PermError::Fail(_) => Err(ConfigError::Permission(p.module(), condition))?,
            },
        }
    }

    Ok(())
}

pub fn register_dbus(per: &[Box<dyn Dbus>], args: &mut ExecutionArgs) -> Result<()> {
    for p in per.iter() {
        p.register(args);
    }

    Ok(())
}
