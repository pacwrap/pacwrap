use alpm::{Alpm, 
    CommitResult, 
    FileConflictType, 
    PrepareResult};
use dialoguer::console::Term;
use simplebyteunit::simplebyteunit::{SI, ToByteUnit};

use crate::{sync::{
    query_event::{self, QueryCallback},
    progress_event::{self, ProgressEvent},
    dl_event::{DownloadCallback, self}}, 
    exec::utils::execute_in_container, 
    utils::{print_error, print_warning}, 
    config::InstanceType, constants::{RESET, BOLD, BOLD_WHITE, DIM}};

use crate::utils::prompt::prompt;
use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator,
    TransactionFlags,
    Result, 
    Error};

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

        if ! handle.get_mode().bool() || ag.flags().intersects(TransactionFlags::DATABASE_ONLY | TransactionFlags::FORCE_DATABASE) {
            summary(handle.alpm());

            if ag.flags().contains(TransactionFlags::PREVIEW) {
                return state_transition(&self.state, handle); 
            } 

            if ! ag.flags().contains(TransactionFlags::NO_CONFIRM) {
                let action = ag.action().as_str();
                let query = format!("Proceed with {action}?");

                if let Err(_) = prompt("::", format!("{}{query}{}", *BOLD, *RESET), true) {
                    return state_transition(&self.state, handle);
                }
            }
   
            handle.alpm().set_question_cb(QueryCallback, query_event::questioncb);
        }

        handle.alpm().set_progress_cb(ProgressEvent::new(ag.action()), progress_event::callback(&self.state));

        if let Err(error) = handle.alpm_mut().trans_commit() {  
            erroneous_transaction(error)? 
        }

        if self.keyring {
            ag.keyring_update(inshandle);        
        }

        if let TransactionState::Commit(_) = self.state {
            ag.sync_filesystem(inshandle);
            execute_ldconfig(inshandle);
        }

        handle.mark_depends();
        handle.apply_configuration(inshandle, ag.flags().intersects(TransactionFlags::CREATE)); 
        ag.set_updated(instance.clone());
        ag.logger().log(format!("container {instance}'s {state} transaction complete")).ok();
        state_transition(&self.state, handle)
    }
}

fn state_transition<'a>(state: &TransactionState, handle: &mut TransactionHandle) -> Result<TransactionState> {
    handle.alpm_mut().trans_release().unwrap(); 

    Ok(match state {
        TransactionState::Commit(_) => TransactionState::Complete,
        TransactionState::CommitForeign => TransactionState::Stage,
        _ => unreachable!()
    })
}

fn execute_ldconfig(inshandle: &InstanceHandle) {
    if let InstanceType::DEP = inshandle.metadata().container_type() {
        return;
    }

    execute_in_container(inshandle, vec!("ldconfig"));
}

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
        handle.set_dl_cb(DownloadCallback::new(download as u64, files_to_download), dl_event::download_event);
    }

    println!();
}

fn erroneous_transaction<'a>(error: (CommitResult<'a>, alpm::Error)) -> Result<()> {
    match error.0 {
        CommitResult::FileConflict(file) => {
            for conflict in file {
                match conflict.conflict_type() {
                    FileConflictType::Filesystem => {
                        let file = conflict.file();
                        let target = conflict.target();
                        print_warning(format!("{}: '{}' already exists.", target, file));
                    },
                    FileConflictType::Target => {
                        let file = conflict.file();
                        let target = format!("{}{}{}",*BOLD_WHITE, conflict.target(), *RESET);
                        if let Some(conflicting) = conflict.conflicting_target() { 
                            let conflicting = format!("{}{conflicting}{}", *BOLD_WHITE, *RESET);
                            print_warning(format!("{conflicting}: '{target}' is owned by {file}")); 
                        } else {
                            print_warning(format!("{target}: '{file}' is owned by foreign target"));
                        }
                    },
                }
            }

            Err(Error::TransactionFailure("Conflict within container filesystem".into()))?
        },
        CommitResult::PkgInvalid(p) => {
            for pkg in p.iter() {
                let pkg = format!("{}{pkg}{}", *BOLD_WHITE, *RESET);
                print_error(format!("Invalid package: {}", pkg)); 
            }
        },
        _ => ()
    }

    Err(Error::TransactionFailure(error.1.to_string()))
}

fn erroneous_preparation<'a>(error:  (PrepareResult<'a>, alpm::Error)) -> Result<()> {  
    match error.0 {
        PrepareResult::PkgInvalidArch(list) => {
        for package in list.iter() {
                print_error(format!("Invalid architecture {}{}{} for {}{}{}", *BOLD, package.arch().unwrap(), *RESET, *BOLD, package.name(), *RESET));
            }
        },
        PrepareResult::UnsatisfiedDeps(list) => {
            for missing in list.iter() {
                print_error(format!("Unsatisifed dependency {}{}{} for target {}{}{}", *BOLD, missing.depend(), *RESET, *BOLD, missing.target(), *RESET));
            }
        },
        PrepareResult::ConflictingDeps(list) => {
            for conflict in list.iter() {
                print_error(format!("Conflict between {}{}{} and {}{}{}: {}", *BOLD, conflict.package1(), *RESET, *BOLD, conflict.package2(), *RESET, conflict.reason()));
            }
        },
        _ => (),
    }
        
    Err(Error::PreparationFailure(error.1.to_string()))
}
