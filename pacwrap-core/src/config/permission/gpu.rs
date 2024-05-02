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

use std::{fs::read_dir, sync::OnceLock};

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        permission::{Condition::Success, PermError::Fail, *},
        Permission,
    },
    exec::args::ExecutionArgs,
    Error,
    ErrorGeneric,
};

static GPU_DEV: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Graphics;

#[typetag::serde(name = "gpu")]
impl Permission for Graphics {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        let gpu_dev = populate_dev().map_err(|f| {
            f.error();
            Fail(format!("No graphics devices are available."))
        })?;

        if GPU_DEV.get_or_init(|| gpu_dev).is_empty() {
            Err(Fail(format!("No graphics devices are available.")))?
        }

        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        for dev in GPU_DEV.get().expect("Uninitialized device array").iter() {
            args.dev(dev);
        }
    }

    fn module(&self) -> &'static str {
        "gpu"
    }
}

fn populate_dev() -> Result<Vec<String>, Error> {
    Ok(read_dir("/dev/")
        .prepend_io(|| format!("/dev"))?
        .into_iter()
        .filter_map(|f| {
            f.map_or_else(
                |_| None,
                |f| {
                    let file = f.file_name();
                    let dev = file.to_str().unwrap();

                    (dev.starts_with("nvidia") || dev == "dri").then_some(format!("/dev/{}", dev))
                },
            )
        })
        .collect::<Vec<String>>())
}
