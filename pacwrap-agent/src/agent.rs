/*
 * pacwrap-agent
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
use std::{
    env,
    fs::{self, File},
    io::ErrorKind::NotFound,
    os::unix::prelude::FileExt,
};

use serde::Deserialize;

use pacwrap_core::{
    config::Global,
    constants::{VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH},
    err,
    log::{Level, Logger},
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
    utils::{bytebuffer::ByteBuffer, print_warning},
    Error,
    ErrorGeneric,
    Result,
};

use crate::error::AgentError;

const AGENT_PARAMS: &str = "/mnt/agent_params";

pub fn transact() -> Result<()> {
    let mut header = ByteBuffer::with_capacity(7).read();
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

    file.read_exact_at(header.as_slice_mut(), 0).prepend_io(|| AGENT_PARAMS.into())?;
    decode_header(&mut header)?;

    let params: TransactionParameters = deserialize(&mut file)?;
    let config: Global = deserialize(&mut file)?;
    let alpm_remotes: AlpmConfigData = deserialize(&mut file)?;
    let mut metadata: TransactionMetadata = deserialize(&mut file)?;
    let handle = TransactionHandle::new(&mut metadata);
    let (transflags, ..) = handle.metadata().retrieve_flags();
    let alpm = sync::instantiate_alpm_agent(&config, &alpm_remotes, &transflags.expect("TransactionFlags"));
    let mut handle = handle.alpm_handle(alpm).config(&config).agent();
    let mut logger = Logger::new("pacwrap-agent").location("/mnt/share/pacwrap.log")?;

    if let Err(err) = conduct_transaction(&config, &mut logger, &mut handle, params) {
        handle.release();
        logger.log(Level::Error, &format!("Transaction Error: {}", err))?;
        return Err(err);
    }

    Ok(())
}

fn conduct_transaction(
    config: &Global,
    logger: &mut Logger,
    handle: &mut TransactionHandle,
    agent: TransactionParameters,
) -> Result<()> {
    let flags = handle.metadata().retrieve_flags();
    let mode = agent.mode();
    let action = agent.action();
    let config = config.config();
    let pkind = config.progress();
    let bytes = agent.bytes();
    let files = agent.files();

    if let Err(error) = handle.alpm_mut().trans_init(flags.1.expect("ALPM TransFlag")) {
        err!(SyncError::InitializationFailure(error.to_string()))?
    }

    handle.ignore(&mut None)?;

    if let TransactionType::Upgrade(upgrade, downgrade, _) = action {
        if upgrade {
            handle.alpm().sync_sysupgrade(downgrade).expect("ALPM sync_sysupgrade")
        }
    }

    handle.prepare(&action, &flags.0.expect("TransactionFlags"))?;

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

    handle.alpm_mut().trans_release().expect("ALPM trans_release");
    handle.mark_depends();

    if let Err(error) = fs::copy("/etc/ld.so.cache", "/mnt/fs/etc/ld.so.cache") {
        if error.kind() != NotFound {
            let message = &format!("Failed to propagate ld.so.cache: {}", error);

            print_warning(message);
            logger.log(Level::Warn, message)?;
        }
    }

    Ok(())
}

fn decode_header(buffer: &mut ByteBuffer) -> Result<()> {
    let magic = buffer.read_le_32();
    let major: (u8, u8) = (*VERSION_MAJOR as u8, buffer.read_byte());
    let minor: (u8, u8) = (*VERSION_MINOR as u8, buffer.read_byte());
    let patch: (u8, u8) = (*VERSION_PATCH as u8, buffer.read_byte());

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
