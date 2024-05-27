/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
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

use pacwrap_core::{
    config::{cache, ConfigError, Container, ContainerHandle, ContainerType, ContainerVariables},
    constants::{ARROW_CYAN, ARROW_GREEN, BOLD, RESET},
    err,
    sync::instantiate_container,
    utils::{
        arguments::{InvalidArgument, Operand},
        Arguments,
    },
    Error,
    Result,
};

pub fn link(args: &mut Arguments) -> Result<()> {
    let (mut dest, mut src, mut new) = (None, None, false);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Value(val) | Operand::ShortPos(_, val) | Operand::LongPos(_, val) =>
                if let None = dest {
                    dest = Some(val);
                } else if let None = src {
                    src = Some(val);
                } else {
                    args.invalid_operand()?;
                },
            Operand::Long("new") | Operand::Short('n') => new = true,
            _ => args.invalid_operand()?,
        }
    }

    let dest = match dest {
        Some(dest) => dest,
        None => return err!(InvalidArgument::TargetUnspecified),
    };
    let src = match src {
        Some(src) => src,
        None => return err!(InvalidArgument::TargetUnspecified),
    };
    let cache = cache::populate()?;
    let dest_handle = cache.get_instance(dest)?;
    let src_handle = match cache.get_instance_option(src) {
        Some(src) => err!(ConfigError::AlreadyExists(src.vars().instance().into()))?,
        None =>
            if new {
                let container = Container::new(ContainerType::Symbolic, vec![], vec![]);
                let container_vars = ContainerVariables::new(src);

                ContainerHandle::new(container, container_vars)
            } else {
                let container_vars = ContainerVariables::new(src);
                let mut deps = dest_handle.metadata().dependencies();
                let mut handle = ContainerHandle::from(dest_handle, container_vars);

                deps.push(dest_handle.vars().instance());
                handle.metadata_mut().set_type(ContainerType::Symbolic);
                handle.metadata_mut().set_metadata(deps, vec![]);
                handle
            },
    };

    instantiate_container(&src_handle)?;
    src_handle.save()?;
    eprintln!(
        "{} Created symbolic container '{}{src}{}' {} '{}{dest}{}'.",
        *ARROW_GREEN, *BOLD, *RESET, *ARROW_CYAN, *BOLD, *RESET
    );
    Ok(())
}
