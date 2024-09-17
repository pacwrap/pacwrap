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

use std::env;

use serde::{Deserialize, Serialize};

use crate::{
    config::{
        permission::{Condition::Success, *},
        Permission,
    },
    exec::args::ExecutionArgs,
    utils::print_warning,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    #[serde(skip_serializing_if = "String::is_empty", default)]
    var: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    set: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    variables: Vec<Var>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Var {
    var: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    set: String,
}

#[typetag::serde(name = "env")]
impl Permission for Environment {
    fn check(&self) -> Result<Option<Condition>, PermError> {
        Ok(Some(Success))
    }

    fn register(&self, args: &mut ExecutionArgs) {
        if !self.var.is_empty() {
            let set = env_var(&self.var, &self.set);
            args.env(&self.var, &set);
        }

        for v in self.variables.iter() {
            let set = env_var(&v.var, &v.set);
            args.env(&v.var, &set);
        }
    }

    fn module(&self) -> &'static str {
        "env"
    }
}

fn env_var(var: &String, set: &String) -> String {
    if !set.is_empty() {
        return set.to_owned();
    }

    match env::var(var) {
        Ok(env) => env,
        Err(_) => {
            print_warning(&format!("Environment variable {} is unset.", var));
            "".into()
        }
    }
}
