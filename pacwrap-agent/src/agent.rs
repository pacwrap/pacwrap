use alpm::TransFlag;
use pacwrap_lib::sync::query_event;
use std::io::Stdin;
use std::process::exit;

use pacwrap_lib::constants::{BOLD, RESET};
use pacwrap_lib::sync::{self, AlpmConfigData,
    progress_event::{self, ProgressEvent}, 
    utils::{erroneous_transaction, erroneous_preparation}, 
    transaction::{TransactionMetadata,
        TransactionHandle,
        TransactionType, 
        TransactionFlags,
        Error, Result}};

use pacwrap_lib::utils::print_error;
use serde::Deserialize;

pub fn transact() {
    let mut stdin = std::io::stdin();
    let meta: TransactionMetadata = deserialize_stdin(&mut stdin);
    let alpm_remotes: AlpmConfigData = deserialize_stdin(&mut stdin);
    let mode: TransactionType = deserialize_stdin(&mut stdin);
    let alpm = sync::instantiate_alpm_agent(&alpm_remotes);
    let mut handle = TransactionHandle::new(alpm, meta);


    println!("[DEBUG]: Running agent in container");

    match conduct_transaction(&mut handle, mode) {
        Ok(_) => handle.release(), 
        Err(error) => {
            handle.release();
            error.message();
            exit(1);
        },
    }

    println!("[DEBUG]: Agent exiting.");
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

    handle.mark_depends();

    Ok(())
}
