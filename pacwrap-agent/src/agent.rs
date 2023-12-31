use std::{fs::{self, File}, io::ErrorKind::NotFound, os::unix::prelude::FileExt, env};

use serde::Deserialize;

use pacwrap_core::{err,
    Error,
    sync::{self, AlpmConfigData,
        utils::{erroneous_transaction, 
            erroneous_preparation}, 
            transaction::{TransactionHandle,
            TransactionType,
            TransactionMetadata,
            TransactionParameters,
            ErrorKind, 
            MAGIC_NUMBER}, 
    event::{download::{DownloadCallback, download_event}, 
            progress::{ProgressEvent, callback}, 
            query::questioncb}}, 
    utils::{print_warning, read_le_32}};

use crate::error::AgentError;

static AGENT_PARAMS: &'static str = "/tmp/agent_params";

pub fn transact() -> Result<(), Error> {
    let mut header_buffer = vec![0; 7]; 
    let mut file = match File::open(AGENT_PARAMS) {
        Ok(file) => file,
        Err(error) => {
            if let Ok(var) = env::var("SHELL") {
                if ! var.is_empty() {
                    err!(AgentError::DirectExecution)?
                }
            }

            err!(AgentError::IOError(AGENT_PARAMS, error.kind()))?
        },
    }; 

    if let Err(error) = file.read_exact_at(&mut header_buffer, 0) {
        err!(AgentError::IOError(AGENT_PARAMS, error.kind()))?
    }
    
    decode_header(&header_buffer)?;

    let params: TransactionParameters = deserialize(&mut file)?; 
    let alpm_remotes: AlpmConfigData = deserialize(&mut file)?; 
    let mut metadata: TransactionMetadata = deserialize(&mut file)?; 
    let alpm = sync::instantiate_alpm_agent(&alpm_remotes);
    let mut handle = TransactionHandle::new(alpm, &mut metadata);

    if let Err(error) = conduct_transaction(&mut handle, params) {
        err!(error)?
    } 

    Ok(())
}

fn conduct_transaction(handle: &mut TransactionHandle, agent: TransactionParameters) -> Result<(), ErrorKind> {
    let flags = handle.retrieve_flags(); 
    let mode = agent.mode();
    let action = agent.action();

    if let Err(error) = handle.alpm_mut().trans_init(flags.1.unwrap()) {
        Err(ErrorKind::InitializationFailure(error.to_string().into()))?
    }

    handle.ignore();  

    if let TransactionType::Upgrade(upgrade, downgrade, _) = action {  
        if upgrade {
            handle.alpm().sync_sysupgrade(downgrade).unwrap();
        }    
    }

    handle.prepare(&action, &flags.0.unwrap())?;

    if let Err(error) = handle.alpm_mut().trans_prepare() {
        erroneous_preparation(error)?
    }

    handle.alpm().set_progress_cb(ProgressEvent::new(&action), callback(&mode));
    handle.alpm().set_question_cb((), questioncb); 
    handle.alpm().set_dl_cb(DownloadCallback::new(agent.bytes(), agent.files()), download_event);

    if let Err(error) = handle.alpm_mut().trans_commit() {
        erroneous_transaction(error)?
    }

    handle.alpm_mut().trans_release().unwrap();
    handle.mark_depends();

    if let Err(error) = fs::copy("/etc/ld.so.cache", "/mnt/etc/ld.so.cache") {
        match error.kind() {
            NotFound => (),_ => print_warning(format!("Failed to propagate ld.so.cache: {}", error)), 
        }
    }

    Ok(())
}

fn decode_header(buffer: &Vec<u8>) -> Result<(), Error> {
    let magic = read_le_32(&buffer, 0);
    let major: (u8, u8) = (env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(), buffer[4]);
    let minor: (u8, u8) = (env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(), buffer[5]);
    let patch: (u8, u8) = (env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(), buffer[6]);

    if magic != MAGIC_NUMBER {
        err!(AgentError::InvalidMagic(magic, MAGIC_NUMBER))?
    }

    if major.0 != major.1 || minor.0 != minor.1 || patch.0 != patch.1 {
        err!(AgentError::InvalidVersion(major.0,minor.0,patch.0,major.1,minor.1,patch.1))?;
    }

    Ok(())
}

fn deserialize<T: for<'de> Deserialize<'de>>(stdin: &mut File) -> Result<T, Error> {
    match bincode::deserialize_from::<&mut File, T>(stdin) {
        Ok(meta) => Ok(meta),
        Err(error) => err!(AgentError::DeserializationError(error.as_ref().to_string()))
    }
}
