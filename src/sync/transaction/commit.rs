use console::{style, Term};
use alpm::{Alpm, 
    CommitResult, 
    FileConflictType, 
    PrepareResult};
use simplebyteunit::simplebyteunit::{SI, ToByteUnit};

use crate::{sync::{
    query_event::{self, QueryCallback},
    progress_event::{self, ProgressCallback},
    dl_event::{DownloadCallback, self}}, 
    exec::utils::execute_in_container, 
    utils::{print_error, print_warning}};

use crate::utils::prompt::prompt;
use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator,
    TransactionFlags,
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

fn keyring_update(ag: &mut TransactionAggregator, inshandle: &InstanceHandle) {
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--populate", "archlinux"));
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--updatedb"));
    ag.set_keyring_synced();
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
   
    print!("\n{}", style(format!("{preface}")).bold());

    for pkg_sync in packages { 
        let pkg = match handle.localdb().pkg(pkg_sync.name()) {
            Ok(pkg) => pkg, Err(_) => pkg_sync,
        };
        let output = format!("{}-{} ", pkg.name(), style(pkg_sync.version()).dim()); 
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
    println!("{}: {}", style(total_str).bold(), installed_size.to_byteunit(SI));  

    let net = installed_size-installed_size_old;

    if net != 0 {
        println!("{}: {}", style("Net Upgrade Size").bold(), net.to_byteunit(SI)); 
    }

    if download > 0 {
        println!("{}: {}", style("Total Download Size").bold(), download.to_byteunit(SI));
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
                        let target = style(conflict.target()).bold().white();
                        if let Some(conflicting) = conflict.conflicting_target() { 
                            let conflicting = style(conflicting).bold().white();
                            print_warning(format!("{}: '{}' is owned by {}", target, file, conflicting)); 
                        } else {
                            print_warning(format!("{}: '{}' is owned by foreign target", target, file));
                        }
                    },
                }
            }

            Err(Error::TransactionFailure("Conflict within container filesystem".into()))?
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
