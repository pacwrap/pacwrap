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

use std::{fs::read_dir, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        permission::{
            Condition::{self, *},
            PermError::{self, *},
        },
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
            Fail("No graphics devices are available.".into())
        })?;
        let nvidia = !gpu_dev.iter().filter(|a| a.contains("nvidia")).collect::<Vec<_>>().is_empty();

        if GPU_DEV.get_or_init(|| gpu_dev).is_empty() {
            Err(Fail("No graphics devices are available.".into()))?
        }

        if nvidia && !Path::new("/sys/module/nvidia").exists() {
            return Ok(Some(SuccessWarn("'/sys/module/nvidia': Device module unavailable.".into())));
        }

        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        let gpu_dev = GPU_DEV.get().expect("Uninitialized device array");
        let nvidia = !gpu_dev.iter().filter(|a| a.contains("nvidia")).collect::<Vec<_>>().is_empty();

        if nvidia && Path::new("/sys/module/nvidia").exists() {
            args.robind("/sys/module/nvidia", "/sys/module/nvidia")
        }

        for dev in gpu_dev {
            args.dev(dev);
        }
    }

    fn module(&self) -> &'static str {
        "gpu"
    }
}

fn populate_dev() -> Result<Vec<String>, Error> {
    Ok(read_dir("/dev/")
        .prepend_io(|| "/dev".into())?
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
