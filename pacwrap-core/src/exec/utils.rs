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

use std::{process::Child, io::Read};

use os_pipe::{PipeReader, PipeWriter};
use serde_yaml::Value;

use crate::{constants::BWRAP_EXECUTABLE, config::InstanceHandle, ErrorKind, error::*, err};

pub fn execute_in_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<()> {
    match super::fakeroot_container(ins, arguments) {
        Ok(mut child) => match child.wait() {
            Ok(_) => Ok(()),
            Err(err) => err!(ErrorKind::ProcessWaitFailure(BWRAP_EXECUTABLE, err.kind()))
        },
        Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())),
    }
}

pub fn bwrap_json(mut reader: PipeReader, writer: PipeWriter) -> Result<i32> { 
    let mut output = String::new();
  
    drop(writer);
    reader.read_to_string(&mut output).unwrap();    
   
    match serde_yaml::from_str::<Value>(&output) {
        Ok(value) => match value["child-pid"].as_u64() {
            Some(value) => Ok(value as i32), 
            None => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
        },
        Err(_) => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
    }
}

pub fn handle_process(name: &str, result: std::result::Result<Child, std::io::Error>) -> Result<()> {
    match result {
        Ok(child) => Ok(wait_on_process(child)),
        Err(error) => err!(ErrorKind::IOError(name.into(), error.kind())),
    }
}

fn wait_on_process(mut child: Child) { 
    child.wait().ok(); 
}
