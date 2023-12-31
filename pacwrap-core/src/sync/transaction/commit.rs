use std::{fs::File, path::Path};

use alpm::Alpm;
use dialoguer::console::Term;
use serde::Serialize;
use simplebyteunit::simplebyteunit::{SI, ToByteUnit};

use crate::{exec::transaction_agent, 
    sync::{DEFAULT_ALPM_CONF, utils::erroneous_preparation, self}, 
    utils::prompt::prompt,
    constants::{PACWRAP_AGENT_FILE, RESET, BOLD, DIM},
    config::InstanceHandle,
};

use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator,
    TransactionFlags,
    TransactionParameters,
    Result, 
    ErrorKind};


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
                TransactionState::CommitForeign => return state_transition(&self.state, handle, false),
                _ => unreachable!()
            }
        } 

        if let Err(error) = handle.alpm_mut().trans_prepare() {
            erroneous_preparation(error)?
        }

        let result = confirm(&self.state, ag, handle);
        let download = result.1.unwrap_or((0,0));

        if let Some(result) = result.0 {
            return result;
        }

        handle.set_alpm(None); 
        write_agent_params(ag, handle, download)?; 
 
        let mut agent = match transaction_agent(inshandle) {
            Ok(child) => child,
            Err(error) => Err(ErrorKind::TransactionFailure(format!("Execution of agent failed: {}", error)))?,      
        };

        match agent.wait() {
            Ok(exit_status) => match exit_status.code().unwrap_or(-1) {
                0 => {
                    if self.keyring {
                        ag.keyring_update(inshandle)?;
                    }

                    handle.set_alpm(Some(sync::instantiate_alpm(inshandle))); 
                    handle.apply_configuration(inshandle, ag.flags().intersects(TransactionFlags::CREATE)); 
                    ag.logger().log(format!("container {instance}'s {state} transaction complete")).ok();
                    state_transition(&self.state, handle, true)
                },
                1 => Err(ErrorKind::TransactionFailureAgent),
                2 => Err(ErrorKind::ParameterAcquisitionFailure),
                3 => Err(ErrorKind::DeserializationFailure), 
                4 => Err(ErrorKind::InvalidMagicNumber),
                5 => Err(ErrorKind::AgentVersionMismatch),
                _ => Err(ErrorKind::TransactionFailure(format!("Generic failure of agent: Exit code {}", exit_status.code().unwrap_or(-1))))?,  
            },
            Err(error) => Err(ErrorKind::TransactionFailure(format!("Execution of agent failed: {}", error)))?,
        }
    } 
}

fn write_agent_params(ag: &TransactionAggregator, handle: &TransactionHandle, download: (u64, usize)) -> Result<()> {
    let f = match File::create(Path::new(*PACWRAP_AGENT_FILE)) {
        Ok(f) => f,
        Err(error) => Err(ErrorKind::IOError((*PACWRAP_AGENT_FILE).into(), error.kind()))?
    };

    serialize(&TransactionParameters::new(*ag.action(), *handle.get_mode(), download.0, download.1), &f)?;  
    serialize(&*DEFAULT_ALPM_CONF, &f)?; 
    serialize(handle.metadata(), &f)?; 
    Ok(())
}

fn serialize<T: for<'de> Serialize>(input: &T, file: &File) -> Result<()> { 
    match bincode::serialize_into::<&File, T>(file, input) {
        Ok(()) => Ok(()),
        Err(error) => Err(ErrorKind::TransactionFailure(format!("Agent data serialization failed: {}", error))),
    }
}

fn confirm(state: &TransactionState, ag: &mut TransactionAggregator, handle: &mut TransactionHandle) -> (Option<Result<TransactionState>>, Option<(u64, usize)>) {
    let mut download = None;

    if ! handle.get_mode().bool() || ag.flags().intersects(TransactionFlags::DATABASE_ONLY | TransactionFlags::FORCE_DATABASE) {
        download = Some(summary(handle.alpm()));

        if ag.flags().contains(TransactionFlags::PREVIEW) {
            return (Some(state_transition(state, handle, false)), None); 
        } 

        if ! ag.flags().contains(TransactionFlags::NO_CONFIRM) {
            let action = ag.action().as_str();
            let query = format!("Proceed with {action}?");

            if let Err(_) = prompt("::", format!("{}{query}{}", *BOLD, *RESET), true) {
                return (Some(state_transition(state, handle, false)), None);
            }
        } 
    }

    handle.alpm_mut().trans_release().ok();
    (None, download)
}

fn state_transition<'a>(state: &TransactionState, handle: &mut TransactionHandle, updated: bool) -> Result<TransactionState> {
    handle.alpm_mut().trans_release().ok();
 
    Ok(match state {
        TransactionState::Commit(_) => TransactionState::Complete(updated),
        TransactionState::CommitForeign => TransactionState::Stage,
        _ => unreachable!()
    })
}

fn summary(handle: &Alpm) -> (u64, usize) { 
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
    }

    println!();
    (download as u64, files_to_download)
}
