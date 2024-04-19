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

use std::{fs::read_dir, path::Path};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        permission::{Condition::Success, PermError::Fail, *},
        Permission,
    },
    exec::args::ExecutionArgs,
};

lazy_static! {
    static ref GPU_DEV: Vec<String> = populate_dev();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Graphics;

#[typetag::serde(name = "gpu")]
impl Permission for Graphics {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        if !Path::new("/dev").exists() {
            Err(Fail(format!("/dev is inaccessible.")))?
        }

        if GPU_DEV.len() == 0 {
            Err(Fail(format!("No graphics devices are available.")))?
        }

        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        for dev in GPU_DEV.iter() {
            args.dev(dev);
        }
    }

    fn module(&self) -> &'static str {
        "gpu"
    }
}

fn populate_dev() -> Vec<String> {
    let mut vec: Vec<String> = Vec::new();
    if let Ok(dir) = read_dir("/dev") {
        for f in dir {
            if let Ok(f) = f {
                let file = f.file_name();
                let dev = file.to_str().unwrap();
                if dev.starts_with("nvidia") || dev == "dri" {
                    vec.push(format!("/dev/{}", dev));
                }
            }
        }
    }

    vec
}
