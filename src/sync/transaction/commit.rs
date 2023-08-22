use console::{style, Term};
use alpm::{Alpm,
    TransFlag, 
    PackageReason,
    PrepareResult, 
    CommitResult, 
    FileConflictType};

use crate::{sync::{
    query_event::{self, QueryCallback},
    progress_event::{self, ProgressCallback},
    utils::{get_local_package, format_unit}, 
    dl_event::{DownloadCallback, self}}, 
    exec::utils::execute_in_container,
    utils::print_error, config::InstanceType};
use crate::utils::prompt::prompt;
use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionType, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator};

pub struct Commit {
    state: TransactionState
}

impl Transaction for Commit { 
    fn new(new: TransactionState) -> Box<Self> {
        Box::new(Self { 
            state: new 
        })
    }

    fn engage(&mut self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> TransactionState {
        let mut set_depends: Vec<String> = Vec::new();
        let mut keyring = None;
        let instance = inshandle.vars().instance();
        let dbonly = match self.state {
            TransactionState::Commit(dbonly) => dbonly, _ => true
        };
        let dep = match self.state {
            TransactionState::Commit(_) => false, _ => true
        };
        let flags = match dbonly { 
            false => TransFlag::NO_DEP_VERSION,
            true => TransFlag::NO_DEP_VERSION | TransFlag::DB_ONLY
        };

        if let Err(error) = handle.alpm().trans_init(flags) {
            return TransactionState::Complete(Err(format!("Failure to initialize transaction: {}.", error.to_string()))); 
        }

        ag.action().action_message(dbonly);
        handle.db(dep);
        handle.ignore();

        match ag.action() {
            TransactionType::Upgrade(upgrade) => { 
                if *upgrade {
                    handle.alpm().sync_sysupgrade(false).unwrap();
                    handle.sync();
                }

                match handle.prepare_add() {
                    Ok(vec) => set_depends = vec,
                    Err(error) => return erroneous_state_transition(handle, error), 
                }


                if let InstanceType::BASE = inshandle.metadata().container_type() {
                    if ! ag.is_keyring_synced() {
                        keyring = handle.alpm()
                            .trans_add()
                            .iter()
                            .find_map(|a| Some(a.name() == "archlinux-keyring"));
                    }
                }
            },
            TransactionType::Remove(depends, cascade) => {
                if let Err(error) = handle.prepare_removal(*depends, *cascade) { 
                    return erroneous_state_transition(handle, error);
                }            
            },
        }

        if ! handle.trans_ready(&ag.action()) {
            return match self.state { 
                TransactionState::CommitForeign => state_transition(&self.state, handle, ag),
                _ => erroneous_state_transition(handle, "Nothing to do.".into())
            }
        } 

        if let Err(error) = handle_preparation(handle.alpm_mut().trans_prepare()) { 
            return erroneous_state_transition(handle, format!("Failure to prepare transaction: {}.", error));
        }

        if ! dbonly || ag.is_database_only() || ag.is_database_force() {
            summary(handle.alpm());

            if ag.is_preview() {
                return state_transition(&self.state, handle, ag); 
            } 

            if ! ag.skip_confirm() {
                let action = ag.action().as_str();
                let query = format!("Proceed with {}?", action);
                if let Err(_) = prompt("::", format!("{}", style(query).bold()), true) {
                    return state_transition(&self.state, handle, ag);
                }
            }

            handle.alpm().set_question_cb(QueryCallback, query_event::questioncb);
            handle.alpm().set_progress_cb(ProgressCallback::new(), progress_event::progress_event);
        }

        if let Err(error) = handle_transaction(handle.alpm_mut().trans_commit()) {
            return erroneous_state_transition(handle, format!("Failure to commit transaction: {}", error));
        }

        for pkg in set_depends {
            if let Some(mut pkg) = get_local_package(handle.alpm(), pkg.as_str()) {
                pkg.set_reason(PackageReason::Depend).unwrap();
            }
        }

        if if let Some(bool) = keyring { bool } else { false } {
            keyring_update(inshandle);
            ag.set_keyring_synced();
        }

        ag.set_updated(instance.clone());
        state_transition(&self.state, handle, ag)
    }
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

fn state_transition(state: &TransactionState, handle: &mut TransactionHandle, ag: &mut TransactionAggregator) -> TransactionState {
    handle.alpm_mut().trans_release().unwrap(); 

    match state {
        TransactionState::Commit(_) => TransactionState::Complete(Ok(())),
        TransactionState::CommitForeign => TransactionState::Commit(ag.is_database_only()),
        _ => TransactionState::Complete(Err(format!("Commit state failure: {:?} ", state)))
    }
}

fn erroneous_state_transition(handle: &mut TransactionHandle, error: String) -> TransactionState {
    handle.alpm_mut().trans_release().ok();  
    TransactionState::Complete(Err(error)) 
}

fn handle_transaction<'a>(result: Result<(),(CommitResult<'a>, alpm::Error)>) -> Result<(),String> {
    match result {
        Ok(_) => Ok(()),
        Err(result) => Err(handle_erroneous_transaction(result))
    }
}

fn handle_erroneous_transaction<'a>(result: (CommitResult<'a>, alpm::Error)) -> String {
    match result.0 {
        CommitResult::FileConflict(file) => {
            print_error("Conflicting files in container filesystem:");
            for conflict in file.iter() {
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
        CommitResult::Ok => print_error(format!("{}", result.1))
    }
    result.1.to_string()
}

fn handle_preparation<'a>(result: Result<(), (PrepareResult<'a>, alpm::Error)>) -> Result<(),String> {
    match result {
        Ok(_) => Ok(()),
        Err(result) => Err(handle_erroneous_preparation(result))
    }
}
 
fn handle_erroneous_preparation<'a>(result: (PrepareResult<'a>, alpm::Error)) -> String {
    match result.0 {
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
        PrepareResult::Ok => print_error(format!("{}", result.1))
    }
    return result.1.to_string();
}

fn keyring_update(inshandle: &InstanceHandle) {
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--populate", "archlinux"));
    execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--updatedb"));
}
