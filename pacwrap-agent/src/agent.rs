/*
 * pacwrap-agent
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
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
use std::{
    env,
    fs::{self, File},
    io::ErrorKind::NotFound,
    os::unix::prelude::FileExt,
};

use serde::Deserialize;

use pacwrap_core::{
    config::Global,
    err,
    sync::{
        self,
        event::{
            download::{self, DownloadEvent},
            progress::{self, ProgressEvent},
            query,
        },
        transaction::{TransactionHandle, TransactionMetadata, TransactionParameters, TransactionType, MAGIC_NUMBER},
        utils::{erroneous_preparation, erroneous_transaction},
        AlpmConfigData,
        SyncError,
    },
    utils::{print_warning, read_le_32},
    Error,
    Result,
};

use crate::error::AgentError;

static AGENT_PARAMS: &'static str = "/mnt/agent_params";

pub fn transact() -> Result<()> {
    let mut header_buffer = vec![0; 7];
    let mut file = match File::open(AGENT_PARAMS) {
        Ok(file) => file,
        Err(error) => {
            if let Ok(var) = env::var("SHELL") {
                if !var.is_empty() {
                    err!(AgentError::DirectExecution)?
                }
            }

            err!(AgentError::IOError(AGENT_PARAMS, error.kind()))?
        }
    };

    if let Err(error) = file.read_exact_at(&mut header_buffer, 0) {
        err!(AgentError::IOError(AGENT_PARAMS, error.kind()))?
    }

    decode_header(&header_buffer)?;

    let params: TransactionParameters = deserialize(&mut file)?;
    let config: Global = deserialize(&mut file)?;
    let alpm_remotes: AlpmConfigData = deserialize(&mut file)?;
    let mut metadata: TransactionMetadata = deserialize(&mut file)?;
    let alpm = sync::instantiate_alpm_agent(&config, &alpm_remotes);
    let mut handle = TransactionHandle::new(&config, alpm, &mut metadata);

    conduct_transaction(&config, &mut handle, params)
}

fn conduct_transaction(config: &Global, handle: &mut TransactionHandle, agent: TransactionParameters) -> Result<()> {
    let flags = handle.retrieve_flags();
    let mode = agent.mode();
    let action = agent.action();
    let config = config.config();
    let pkind = config.progress();
    let bytes = agent.bytes();
    let files = agent.files();

    if let Err(error) = handle.alpm_mut().trans_init(flags.1.unwrap()) {
        err!(SyncError::InitializationFailure(error.to_string().into()))?
    }

    handle.ignore(true);

    if let TransactionType::Upgrade(upgrade, downgrade, _) = action {
        if upgrade {
            handle.alpm().sync_sysupgrade(downgrade).unwrap();
        }
    }

    handle.prepare(&action, &flags.0.unwrap())?;

    if let Err(error) = handle.alpm_mut().trans_prepare() {
        erroneous_preparation(error)?
    }

    let progress_cb = ProgressEvent::new().style(pkind.0).configure(&action);
    let download_cb = DownloadEvent::new().style(pkind.1).total(bytes, files).configure(&mode, pkind.1);

    handle.alpm().set_question_cb((), query::callback);
    handle.alpm().set_progress_cb(progress_cb, progress::callback(&mode, pkind.0));
    handle.alpm().set_dl_cb(download_cb, download::callback(pkind.1));

    if let Err(error) = handle.alpm_mut().trans_commit() {
        erroneous_transaction(error)?
    }

    handle.alpm_mut().trans_release().unwrap();
    handle.mark_depends();

    if let Err(error) = fs::copy("/etc/ld.so.cache", "/mnt/fs/etc/ld.so.cache") {
        match error.kind() {
            NotFound => (),
            _ => print_warning(format!("Failed to propagate ld.so.cache: {}", error)),
        }
    }

    Ok(())
}

fn decode_header(buffer: &Vec<u8>) -> Result<()> {
    let magic = read_le_32(&buffer, 0);
    let major: (u8, u8) = (env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(), buffer[4]);
    let minor: (u8, u8) = (env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(), buffer[5]);
    let patch: (u8, u8) = (env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(), buffer[6]);

    if magic != MAGIC_NUMBER {
        err!(AgentError::InvalidMagic(magic, MAGIC_NUMBER))?
    }

    if major.0 != major.1 || minor.0 != minor.1 || patch.0 != patch.1 {
        err!(AgentError::InvalidVersion(major.0, minor.0, patch.0, major.1, minor.1, patch.1))?;
    }

    Ok(())
}

fn deserialize<T: for<'de> Deserialize<'de>>(stdin: &mut File) -> Result<T> {
    match bincode::deserialize_from::<&mut File, T>(stdin) {
        Ok(meta) => Ok(meta),
        Err(error) => err!(AgentError::DeserializationError(error.as_ref().to_string())),
    }
}
