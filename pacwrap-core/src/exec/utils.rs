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

use std::{
    io::Read,
    os::fd::AsRawFd,
    path::Path,
    process::{exit, Child},
    thread,
    time::Duration,
};

use os_pipe::{PipeReader, PipeWriter};
use serde::Serialize;
use serde_yaml::Value;

use crate::{
    config::global,
    constants::BWRAP_EXECUTABLE,
    err,
    error::*,
    exec::{ExecutionError, ExecutionType},
    sync::{
        alpm_config,
        transaction::{TransactionMetadata, TransactionParameters},
        SyncError,
    },
    utils::TermControl,
    ErrorKind,
};

static PROCESS_SLEEP_DURATION: Duration = Duration::from_millis(250);

pub fn wait_on_container(
    mut process: Child,
    term: TermControl,
    bwrap_pid: i32,
    block: bool,
    jobs: Option<Vec<Child>>,
    trap_cb: fn(i32),
    exit_cb: fn() -> Result<()>,
) -> Result<()> {
    trap_cb(bwrap_pid);

    match process.wait() {
        Ok(status) => {
            if block {
                let proc: &str = &format!("/proc/{}/", bwrap_pid);
                let proc = Path::new(proc);

                while proc.exists() {
                    thread::sleep(PROCESS_SLEEP_DURATION);
                }
            }

            if let Some(mut jobs) = jobs {
                for job in jobs.iter_mut() {
                    job.kill().unwrap();
                }
            }

            if let Err(err) = exit_cb() {
                err.warn();
            }

            if let Err(err) = term.reset_terminal() {
                err.warn();
            }

            match status.code() {
                Some(code) => exit(code),
                None => {
                    eprint!("\nbwrap process {status}");
                    println!();
                    exit(ExecutionError::Bwrap(status).code())
                }
            }
        }
        Err(error) => err!(ErrorKind::ProcessWaitFailure(BWRAP_EXECUTABLE, error.kind())),
    }
}

pub fn wait_on_fakeroot(
    exec_type: ExecutionType,
    mut process: Child,
    term: TermControl,
    bwrap_pid: i32,
    trap_cb: Option<fn(i32)>,
) -> Result<()> {
    if let Some(trap) = trap_cb {
        trap(bwrap_pid)
    }

    match process.wait() {
        Ok(status) => {
            if let Err(err) = term.reset_terminal() {
                err.warn();
            }

            match status.code() {
                Some(code) => match (exec_type, code) {
                    (_, 0) => Ok(()),
                    (ExecutionType::Interactive, _) => exit(code),
                    (ExecutionType::NonInteractive, _) => err!(ExecutionError::Container(code)),
                },
                None => match exec_type {
                    ExecutionType::Interactive => {
                        eprint!("\nbwrap process {status}");
                        println!();
                        exit(ExecutionError::Bwrap(status).code())
                    }
                    ExecutionType::NonInteractive => err!(ExecutionError::Bwrap(status)),
                },
            }
        }
        Err(error) => err!(ErrorKind::ProcessWaitFailure(BWRAP_EXECUTABLE, error.kind())),
    }
}

pub fn decode_info_json(mut info_pipe: (PipeReader, PipeWriter)) -> Result<i32> {
    let mut output = String::new();

    drop(info_pipe.1);
    info_pipe.0.read_to_string(&mut output).unwrap();

    match serde_yaml::from_str::<Value>(&output) {
        Ok(value) => match value["child-pid"].as_u64() {
            Some(value) => Ok(value as i32),
            None => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
        },
        Err(_) => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
    }
}

pub fn agent_params(
    reader: &PipeReader,
    writer: &PipeWriter,
    params: &TransactionParameters,
    metadata: &TransactionMetadata,
) -> Result<i32> {
    serialize(params, writer)?;
    serialize(global()?, writer)?;
    serialize(alpm_config()?, writer)?;
    serialize(metadata, writer)?;
    Ok(reader.as_raw_fd())
}

fn serialize<T: for<'de> Serialize>(input: &T, file: &PipeWriter) -> Result<()> {
    match bincode::serialize_into::<&PipeWriter, T>(file, input) {
        Ok(()) => Ok(()),
        Err(error) => err!(SyncError::TransactionFailure(format!("Agent data serialization failed: {}", error))),
    }
}

pub fn handle_process(name: &'static str, result: std::result::Result<Child, std::io::Error>) -> Result<()> {
    match result {
        Ok(child) => wait_on_process(name, child),
        Err(error) => err!(ErrorKind::ProcessInitFailure(name, error.kind())),
    }
}

pub fn wait_on_process(name: &'static str, mut child: Child) -> Result<()> {
    match child.wait() {
        Ok(_) => Ok(()),
        Err(error) => err!(ErrorKind::ProcessWaitFailure(name, error.kind())),
    }
}
