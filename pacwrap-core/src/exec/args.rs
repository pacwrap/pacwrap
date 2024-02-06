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

use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

pub struct ExecutionArgs {
    bind: Vec<String>,
    dev: Vec<String>,
    env: Vec<String>,
    dbus: Vec<String>,
    vars: HashMap<String, String>,
}

//TODO: This entire structure needs to be rethought
impl ExecutionArgs {
    pub fn new() -> Self {
        Self {
            bind: Vec::new(),
            dev: Vec::new(),
            env: Vec::new(),
            dbus: Vec::new(),
            vars: HashMap::new(),
        }
    }

    pub fn dir(&mut self, dest: &str) {
        self.bind.push("--dir".into());
        self.bind.push(dest.into());
    }

    pub fn bind(&mut self, src: &str, dest: &str) {
        self.bind.push("--bind".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn robind(&mut self, src: &str, dest: &str) {
        self.bind.push("--ro-bind".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn symlink(&mut self, src: &str, dest: &str) {
        self.bind.push("--symlink".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn env(&mut self, src: &str, dest: &str) {
        self.env.push("--setenv".into());
        self.env.push(src.into());
        self.env.push(dest.into());

        //TODO: Temporary workaround until structure is rebuilt
        self.vars.insert(src.into(), dest.into());
    }

    pub fn dev(&mut self, src: &str) {
        self.dev.push("--dev-bind-try".into());
        self.dev.push(src.into());
        self.dev.push(src.into());
    }

    pub fn dbus(&mut self, per: &str, socket: &str) {
        self.dbus.push(format!("--{}={}", per, socket));
    }

    pub fn push_env(&mut self, src: &str) {
        self.env.push(src.into());
    }

    pub fn get_bind(&self) -> &Vec<String> {
        &self.bind
    }

    pub fn get_dev(&self) -> &Vec<String> {
        &self.dev
    }

    pub fn get_env(&self) -> &Vec<String> {
        &self.env
    }

    pub fn get_dbus(&self) -> &Vec<String> {
        &self.dbus
    }

    //TODO: Temporary workaround until structure is rebuilt
    pub fn get_var(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }
}

impl Debug for ExecutionArgs {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "bind: {:?}", self.bind)?;
        writeln!(fmter, "env:  {:?}", self.env)?;

        if self.dev.len() > 0 {
            writeln!(fmter, "dev:  {:?}", self.dev)?;
        }

        if self.dbus.len() > 0 {
            writeln!(fmter, "dbus: {:?}", self.dbus)?;
        }

        Ok(())
    }
}
