use console::{style, Term};
use alpm::{Alpm, 
    CommitResult, 
    FileConflictType, 
    PrepareResult};

use crate::{sync::{
    query_event::{self, QueryCallback},
    progress_event::{self, ProgressCallback},
    utils::format_unit, 
    dl_event::{DownloadCallback, self}}, 
    exec::utils::execute_in_container, 
    utils::print_error};

use crate::utils::prompt::prompt;
use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator,
    Result, Error};

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
            keyring: kr
        })
    }

    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        let ready = handle.trans_ready(&ag.action());

        if let Err(_) = ready {
            match self.state { 
                TransactionState::CommitForeign => return state_transition(&self.state, handle),
                TransactionState::Commit(_) => ready?,
                _ => unreachable!()
            }
        } 

        if let Err(error) = handle.alpm_mut().trans_prepare() {
            erroneous_preparation(error)?
        }

        if ! handle.get_mode().bool() || ag.is_database_only() || ag.is_database_force() {
            summary(handle.alpm());

            if ag.is_preview() {
                return state_transition(&self.state, handle); 
            } 

            if ! ag.skip_confirm() {
                let action = ag.action().as_str();
                let query = format!("Proceed with {}?", action);

                if let Err(_) = prompt("::", format!("{}", style(query).bold()), true) {
                    return state_transition(&self.state, handle);
                }
            }

            handle.alpm().set_question_cb(QueryCallback, query_event::questioncb);
            handle.alpm().set_progress_cb(ProgressCallback::new(), progress_event::progress_event);
        }

        if let Err(error) = handle.alpm_mut().trans_commit() {  
            erroneous_transaction(error)? 
        }

        if self.keyring {
            keyring_update(ag, inshandle);
        }

        handle.mark_depends();
        ag.set_updated(instance.clone());
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

fn keyring_update(ag: &mut TransactionAggregator, inshandle: &InstanceHandle) {
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--populate", "archlinux"));
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--updatedb"));
    ag.set_keyring_synced();
}

fn summary(handle: &Alpm) { 
    let remove = if handle.trans_remove().len() > 0 {
        true
    } else {
        false
    };
    let packages = if remove {
        handle.trans_remove()
    } else {
        handle.trans_add()
    };
 
    let size = Term::size(&Term::stdout());
    let mut installed_size_old: i64 = 0;
    let mut installed_size: i64 = 0;
    let mut download: i64 = 0;
    let mut files_to_download: usize = 0;
    let preface = format!("Packages ({}) ", packages.len());
    let mut print_string: String = String::new();
    let line_delimiter = size.1 as isize - preface.len() as isize;
    let mut current_line_len: isize = 0;

    print!("\n{}", style(format!("{}", preface)).bold());

    for val in packages { 
        let pkg_sync = val;
        let pkg = match handle.localdb().pkg(pkg_sync.name()) {
            Ok(pkg) => pkg,
            Err(_) => pkg_sync,
        };
        let output = format!("{}-{} ", pkg.name(), style(pkg_sync.version()).dim()); 
        let download_size = pkg_sync.download_size();

        installed_size_old += pkg.isize();             
        installed_size += pkg_sync.isize();
        
        if download_size > 0 {
            download += download_size;
            files_to_download += 1;
        }

        current_line_len += print_string.len() as isize;
        print_string.push_str(&output);  

        if current_line_len >= line_delimiter { 
            print!("{}\n", print_string);
            print_string = " ".repeat(preface.len());
            current_line_len = 0;
        }
    }

    if print_string.len() > 0 {
        print!("{}\n\n", print_string);
    }
              
    let net = installed_size-installed_size_old;

    if remove {
        println!("{}: {}", style("Total Removed Size").bold(), format_unit(installed_size));  
    } else {
        println!("{}: {}", style("Total Installed Size").bold(), format_unit(installed_size));  
    }

    if net != 0 {
        println!("{}: {}", style("Net Upgrade Size").bold(), format_unit(net)); 
    }

    if download > 0 {
        println!("{}: {}", style("Total Download Size").bold(), format_unit(download));
        handle.set_dl_cb(DownloadCallback::new(download.try_into().unwrap(), files_to_download), dl_event::download_event);
    }

    println!();
}

fn erroneous_transaction<'a>(error: (CommitResult<'a>, alpm::Error)) -> Result<()> {
    match error.0 {
        CommitResult::FileConflict(file) => {
            print_error("Conflicting files in container filesystem:");
            for conflict in file {
                match conflict.conflict_type() {
                    FileConflictType::Filesystem => {
                        let file = conflict.file();
                        let target = conflict.target();
                        println!("{}: '{}' already exists.", target, file);
                    },
                    FileConflictType::Target => {
                        let file = conflict.file();
                        let target = style(conflict.target()).bold().white();
                        if let Some(conflicting) = conflict.conflicting_target() { 
                            let conflicting = style(conflicting).bold().white();
                            println!("{}: '{}' is owned by {}", target, file, conflicting); 
                        } else {
                            println!("{}: '{}' is owned by foreign target", target, file);
                        }
                    },
                }
            }
        },
        CommitResult::PkgInvalid(p) => {
            for pkg in p.iter() {
                let pkg = style(pkg).bold().white();  
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
                print_error(format!("Invalid architecture {} for {}", style(package.arch().unwrap()).bold(), style(package.name()).bold()));
            }
        },
        PrepareResult::UnsatisfiedDeps(list) => {
            for missing in list.iter() {
                print_error(format!("Unsatisifed dependency {} for target {}", style(missing.depend()).bold(), style(missing.target()).bold()));
            }
        },
        PrepareResult::ConflictingDeps(list) => {
            for conflict in list.iter() {
                print_error(format!("Conflict between {} and {}: {}", style(conflict.package1()).bold(), style(conflict.package2()).bold(), conflict.reason()));
            }
        },
        _ => (),
    }
        
    Err(Error::PreparationFailure(error.1.to_string()))
}
