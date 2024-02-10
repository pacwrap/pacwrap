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

use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub enum Argument {
    Directory(String),
    Bind(String, String),
    RoBind(String, String),
    EnvVar(String, String),
    SymbolicLink(String, String),
    Device(String),
    DevFs,
    DieWithParent,
    DisableNamespaces,
    HostNetworking,
    ProcFs,
    NewSession,
    TmpFs,
    UnshareAll,
}

pub struct ExecutionArgs {
    dbus: Vec<String>,
    bind: Vec<Argument>,
    env: Vec<Argument>,
    sys: Vec<Argument>,
}

impl Argument {
    fn to_vec(&self) -> Vec<&str> {
        match self {
            Self::Directory(val) => vec!["--dir", val],
            Self::Bind(src, dest) => vec!["--bind", src, dest],
            Self::RoBind(src, dest) => vec!["--ro-bind", src, dest],
            Self::SymbolicLink(src, dest) => vec!["--symlink", src, dest],
            Self::EnvVar(val, set) => vec!["--setenv", val, set],
            Self::Device(val) => vec!["--dev-bind-try", val, val],
            Self::DevFs => vec!["--dev", "/dev"],
            Self::DieWithParent => vec!["--die-with-parent"],
            Self::DisableNamespaces => vec!["--unshare-user", "--disable-userns"],
            Self::HostNetworking => vec!["--share-net"],
            Self::ProcFs => vec!["--proc", "/proc"],
            Self::NewSession => vec!["--new-session"],
            Self::TmpFs => vec!["--tmpfs", "/tmp"],
            Self::UnshareAll => vec!["--unshare-all"],
        }
    }
}

impl ExecutionArgs {
    pub fn new() -> Self {
        Self {
            dbus: Vec::new(),
            bind: vec![Argument::TmpFs],
            sys: vec![Argument::DevFs, Argument::ProcFs],
            env: vec![Argument::UnshareAll],
        }
    }

    pub fn dir(&mut self, dest: &str) {
        self.bind.push(Argument::Directory(dest.into()));
    }

    pub fn bind(&mut self, src: &str, dest: &str) {
        self.bind.push(Argument::Bind(src.into(), dest.into()));
    }

    pub fn robind(&mut self, src: &str, dest: &str) {
        self.bind.push(Argument::RoBind(src.into(), dest.into()));
    }

    pub fn symlink(&mut self, src: &str, dest: &str) {
        self.bind.push(Argument::SymbolicLink(src.into(), dest.into()));
    }

    pub fn env(&mut self, src: &str, dest: &str) {
        self.env.push(Argument::EnvVar(src.into(), dest.into()));
    }

    pub fn dev(&mut self, src: &str) {
        self.sys.push(Argument::Device(src.into()));
    }

    pub fn dbus(&mut self, per: &str, socket: &str) {
        self.dbus.push(format!("--{}={}", per, socket));
    }

    pub fn push_env(&mut self, arg: Argument) {
        self.env.push(arg);
    }

    pub fn get_dbus(&self) -> Vec<&str> {
        self.dbus.iter().map(|a| a.as_str()).collect()
    }

    pub fn obtain_env(&self, env: &str) -> Option<&str> {
        self.env.iter().find_map(|a| match a {
            Argument::EnvVar(target, var) => match target == env {
                true => Some(var.as_str()),
                false => None,
            },
            _ => None,
        })
    }

    pub fn arguments(&self) -> Vec<&str> {
        let mut vec = Vec::new();

        vec.reserve((self.sys.len() + self.bind.len() + self.env.len()) * 4);

        for values in self.bind.iter().chain(self.sys.iter()).chain(self.env.iter()) {
            vec.extend(values.to_vec());
        }

        vec
    }
}

impl Debug for ExecutionArgs {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "bind: {:?}", self.bind)?;
        writeln!(fmter, "env:  {:?}", self.env)?;

        if self.sys.len() > 2 {
            writeln!(fmter, "sys:  {:?}", self.sys)?;
        }

        if self.dbus.len() > 0 {
            writeln!(fmter, "dbus: {:?}", self.dbus)?;
        }

        Ok(())
    }
}
