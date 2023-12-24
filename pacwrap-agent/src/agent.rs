use std::{fs::{self, File}, process::exit, io::ErrorKind::NotFound, os::unix::prelude::FileExt, env};

use serde::Deserialize;

use pacwrap_core::{sync::{self, AlpmConfigData,
        utils::{erroneous_transaction, 
            erroneous_preparation}, 
            transaction::{TransactionHandle,
            TransactionType,
            TransactionMetadata,
            TransactionParameters,
            Error, 
            Result, MAGIC_NUMBER}, 
    event::{download::{DownloadCallback, download_event}, progress::{ProgressEvent, callback}, query::questioncb}}, 
    utils::{print_error, print_warning, read_le_32}, constants::{RESET, BOLD}};

pub fn transact() {
    let mut header_buffer = vec![0; 7]; 
    let mut file = match File::open("/tmp/agent_params") {
        Ok(file) => file,
        Err(_) => {
            if let Ok(var) = env::var("SHELL") {
                if ! var.is_empty() {
                    print_error("Direct execution of this binary is unsupported.");
                }
            }

            exit(2);
        },
    }; 

    if let Err(error) = file.read_exact_at(&mut header_buffer, 0) {
        print_error(format!("'{}/tmp/agent_params{}': {error}", *BOLD, *RESET));
        exit(3);
    }

    let magic = read_le_32(&header_buffer, 0);
    let major: u8 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
    let minor: u8 = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
    let patch: u8 = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();

    if magic != MAGIC_NUMBER {
        print_error(format!("Magic number {magic} != {MAGIC_NUMBER}"));
        exit(4);
    }

    if major != header_buffer[4] || minor != header_buffer[5] || patch != header_buffer[6] {
        print_error(format!("{major}.{minor}.{patch} != {}.{}.{}", header_buffer[4], header_buffer[5], header_buffer[6]));
        exit(5); 
    }

    let params: TransactionParameters = deserialize(&mut file); 
    let alpm_remotes: AlpmConfigData = deserialize(&mut file); 
    let mut metadata: TransactionMetadata = deserialize(&mut file); 
    let alpm = sync::instantiate_alpm_agent(&alpm_remotes);
    let mut handle = TransactionHandle::new(alpm, &mut metadata);

    if let Err(error) = conduct_transaction(&mut handle, params) {
        print_error(error);
        handle.alpm_mut().trans_release().ok();
        exit(1);
    } 
}

fn conduct_transaction(handle: &mut TransactionHandle, agent: TransactionParameters) -> Result<()> {
    let flags = handle.retrieve_flags(); 
    let mode = agent.mode();
    let action = agent.action();

    if let Err(error) = handle.alpm_mut().trans_init(flags.1.unwrap()) {
        Err(Error::InitializationFailure(error.to_string().into()))?
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

fn deserialize<T: for<'de> Deserialize<'de>>(stdin: &mut File) -> T {
    match bincode::deserialize_from::<&mut File, T>(stdin) {
        Ok(meta) => meta,
        Err(error) => { 
            print_error(format!("Deserilization error: {}", error.as_ref()));
            exit(3);
        }
    }
}
