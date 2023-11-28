use std::{fs, io::Stdin, process::exit};


use alpm::TransFlag;

use pacwrap_core::{constants::{BOLD, RESET},
    utils::print_error,
    sync::{self, AlpmConfigData,
        query_event,
        progress_event::{self, ProgressEvent}, 
        utils::{erroneous_transaction, erroneous_preparation}, 
            transaction::{TransactionMetadata,
            TransactionHandle,
            TransactionType, 
            TransactionFlags,
            Error, 
            Result}}};
use serde::Deserialize;

pub fn transact() {
    let mut stdin = std::io::stdin();
    let meta: TransactionMetadata = deserialize_stdin(&mut stdin);
    let alpm_remotes: AlpmConfigData = deserialize_stdin(&mut stdin);
    let mode: TransactionType = deserialize_stdin(&mut stdin);
    let alpm = sync::instantiate_alpm_agent(&alpm_remotes);
    let mut handle = TransactionHandle::new(alpm, meta);

    if let Err(error) = conduct_transaction(&mut handle, mode) {
        print_error(error);
        handle.alpm_mut().trans_release().ok();
        exit(1);
    }
}

fn deserialize_stdin<T: for<'de> Deserialize<'de>>(stdin: &mut Stdin) -> T {
    match ciborium::from_reader::<T, &mut Stdin>(stdin) {
        Ok(meta) => meta,
        Err(err) => {
            if let ciborium::de::Error::Semantic(_, error) = err {
                match error.contains("integer `10`") {
                    false => print_error(format!("Deserialization failure occurred with input from {}STDIN{}: {error}", *BOLD, *RESET)),
                    true => print_error("Interactive user input is not supported by this program."),
                } 
            }

            exit(1);
        }
    }
}

fn conduct_transaction(handle: &mut TransactionHandle, mode: TransactionType) -> Result<()> {
    let flags = handle.retrieve_flags(); 
    let flag_trans = TransactionFlags::from_bits(flags.0).unwrap();
    let mode_trans = handle.get_mode().clone();

    if let Err(error) = handle.alpm_mut().trans_init(TransFlag::from_bits(flags.1).unwrap()) {
        Err(Error::InitializationFailure(error.to_string().into()))?
    }

    handle.ignore();  

    match mode {
        TransactionType::Upgrade(upgrade, downgrade, _) => {  
            if upgrade {
                handle.alpm().sync_sysupgrade(downgrade).unwrap();
            }

            handle.prepare_add(&flag_trans)?;
        },
        TransactionType::Remove(depends, cascade, explicit) => handle.prepare_removal(depends, cascade, explicit)?,
    }


    if let Err(error) = handle.alpm_mut().trans_prepare() {
        erroneous_preparation(error)?
    }

    handle.alpm().set_progress_cb(ProgressEvent::new(&mode), progress_event::callback(&mode_trans));
    handle.alpm().set_question_cb((), query_event::questioncb);
    //handle.alpm().set_dl_cb(DownloadCallback::new(), dl_event::download_event);

    if let Err(error) = handle.alpm_mut().trans_commit() {
        erroneous_transaction(error)?
    }

    handle.alpm_mut().trans_release().unwrap();
    fs::copy("/etc/ld.so.cache", "/mnt/etc/ld.so.cache").ok();
    handle.mark_depends();
    Ok(())
}
