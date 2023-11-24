use std::process::ChildStdin;

use alpm::Alpm;
use dialoguer::console::Term;
use serde::Serialize;
use crate::{exec::utils::execute_agent, sync::{DEFAULT_ALPM_CONF, utils::erroneous_preparation, self}};
use simplebyteunit::simplebyteunit::{SI, ToByteUnit};

use crate::constants::{RESET, BOLD, DIM};

use crate::utils::prompt::prompt;
use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator,
    TransactionFlags,
    Result, 
    Error};

#[allow(dead_code)]
pub struct Commit {
    state: TransactionState,
    keyring: bool,
}

impl Transaction for Commit { 
    fn new(new: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        let kr = match new { 
            TransactionState::Commit(bool) => bool, _ => false
        };

        Box::new(Self { 
            state: new,
            keyring: kr,
        })
    }

    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        let ready = handle.trans_ready(&ag.action());
        let state = self.state.as_str();

        if let Err(_) = ready {
            match self.state { 
                TransactionState::Commit(_) => ready?,
                TransactionState::CommitForeign => return state_transition(&self.state, handle),
                _ => unreachable!()
            }
        } 

        if let Err(error) = handle.alpm_mut().trans_prepare() {
            erroneous_preparation(error)?
        }

        if let Some(result) = confirm(&self.state, ag, handle) {
            return result;
        }

        handle.set_alpm(None);
  
        match execute_agent(inshandle) {
            Ok(mut child) => {
                let stdin = child.stdin.take().unwrap();

                write_to_stdin(handle.metadata(), &stdin)?;
                write_to_stdin(&*DEFAULT_ALPM_CONF, &stdin)?;
                write_to_stdin(ag.action(), &stdin)?;

                match child.wait() {
                    Ok(exit_status) => match exit_status.code().unwrap_or(0) {
                        1 => Err(Error::AgentError),
                        0 => {
                            if self.keyring {
                                ag.keyring_update(inshandle);
                            }

                            handle.set_alpm(Some(sync::instantiate_alpm(inshandle))); 
                            handle.apply_configuration(inshandle, ag.flags().intersects(TransactionFlags::CREATE)); 
                            //ag.set_updated(instance.clone());
                            ag.logger().log(format!("container {instance}'s {state} transaction complete")).ok();
                            state_transition(&self.state, handle)
                        }, 
                        _ => Err(Error::TransactionFailure(format!("Generic failure of agent: Exit code {}", exit_status.code().unwrap_or(0))))?,  
                    },
                    Err(error) => Err(Error::TransactionFailure(format!("Execution of agent failed: {}", error)))?,
                }
            },
            Err(error) => Err(Error::TransactionFailure(format!("Execution of agent failed: {}", error)))?,     
        }
    } 
}

fn write_to_stdin<T: for<'de> Serialize>(input: &T, stdin: &ChildStdin) -> Result<()> { 
    match ciborium::into_writer::<T, &ChildStdin>(input, stdin) {
        Ok(()) => Ok(()),
        Err(error) => Err(Error::TransactionFailure(format!("Agent data serialization failed: {}", error))),
    }
}

fn confirm(state: &TransactionState, ag: &mut TransactionAggregator, handle: &mut TransactionHandle) -> Option<Result<TransactionState>> {
    if ! handle.get_mode().bool() || ag.flags().intersects(TransactionFlags::DATABASE_ONLY | TransactionFlags::FORCE_DATABASE) {
        summary(handle.alpm());

        if ag.flags().contains(TransactionFlags::PREVIEW) {
            return Some(state_transition(state, handle)); 
        } 

        if ! ag.flags().contains(TransactionFlags::NO_CONFIRM) {
            let action = ag.action().as_str();
            let query = format!("Proceed with {action}?");

            if let Err(_) = prompt("::", format!("{}{query}{}", *BOLD, *RESET), true) {
                return Some(state_transition(state, handle));
            }
        } 
    }

    handle.alpm_mut().trans_release().ok();
    None
}

fn state_transition<'a>(state: &TransactionState, handle: &mut TransactionHandle) -> Result<TransactionState> {
    handle.alpm_mut().trans_release().ok();
 
    Ok(match state {
        TransactionState::Commit(_) => TransactionState::Complete,
        TransactionState::CommitForeign => TransactionState::Stage,
        _ => unreachable!()
    })
}

#[allow(unused_variables)]
fn summary(handle: &Alpm) { 
    let mut installed_size_old: i64 = 0;
    let mut installed_size: i64 = 0;
    let mut download: i64 = 0;
    let mut files_to_download: usize = 0; 
    let mut pkglist: String = String::new();
    let mut current_line_len: usize = 0;
    let remove = if handle.trans_remove().len() > 0 { true } else { false };
    let packages = if remove { handle.trans_remove() } else { handle.trans_add() };
    let total_str = if remove { "Total Removed Size" } else { "Total Installed Size" };
    let size = Term::size(&Term::stdout());
    let preface = format!("Packages ({}) ", packages.len());
    let preface_newline = " ".repeat(preface.len()); 
    let line_delimiter = size.1 as usize - preface.len();
   
    print!("\n{}{preface}{}", *BOLD, *RESET);

    for pkg_sync in packages { 
        let pkg = match handle.localdb().pkg(pkg_sync.name()) {
            Ok(pkg) => pkg, Err(_) => pkg_sync,
        };
        let output = format!("{}-{}{}{} ", pkg.name(), *DIM, pkg_sync.version(), *RESET); 
        let download_size = pkg_sync.download_size();
        let string_len = pkg.name().len() + pkg_sync.version().len() + 2;

        if current_line_len+string_len >= line_delimiter { 
            println!("{pkglist}"); 
            pkglist = preface_newline.clone();
            current_line_len = pkglist.len(); 
        }

        current_line_len += string_len;
        installed_size_old += pkg.isize(); 
        installed_size += pkg_sync.isize();
        
        if download_size > 0 {
            download += download_size;
            files_to_download += 1;
        }

        pkglist.push_str(&output);  
    }

    print!("{pkglist}\n\n");
    println!("{}{total_str}{}: {}", *BOLD, *RESET, installed_size.to_byteunit(SI));  

    let net = installed_size-installed_size_old;

    if net != 0 {
        println!("{}Net Upgrade Size{}: {}", *BOLD, *RESET, net.to_byteunit(SI)); 
    }

    if download > 0 {
        println!("{}Total Download Size{}: {}", *BOLD, *RESET, download.to_byteunit(SI));
        //handle.set_dl_cb(DownloadCallback::new(download as u64, files_to_download), dl_event::download_event);
    }

    println!();
}
