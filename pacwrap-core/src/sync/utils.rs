use alpm::{CommitResult, FileConflictType, Package, Alpm, PrepareResult};

use crate::{sync::transaction::Result, sync::transaction::ErrorKind};
use crate::constants::{BOLD, BOLD_WHITE, RESET};
use crate::utils::{print_error, print_warning};

pub trait AlpmUtils {
    fn get_local_package(&self, pkg: &str) -> Option<Package<'_>>;
    fn get_package(&self, pkg: &str) -> Option<Package<'_>>; 
}

impl AlpmUtils for Alpm {
    fn get_local_package<'a>(&self, pkg: &'a str) -> Option<Package<'_>> {
        if let Ok(pkg) = self.localdb().pkg(pkg) {
            return Some(pkg);
        } else {
            self.localdb()
                .pkgs()
                .iter()
                .find_map(|f| {
                if f.provides()
                        .iter()
                        .filter(|d| pkg == d.name())
                        .count() > 0 {
                    Some(f)
                } else {
                    None
                }  
            })
        }
    }

    fn get_package(&self, pkg: &str) -> Option<Package<'_>> {
        for sync in self.syncdbs() {  
            if let Ok(pkg) = sync.pkg(pkg) {
                return Some(pkg);
            } else {
                let package = sync.pkgs()
                    .iter()
                    .find_map(|f| {
                    if f.provides()
                            .iter()
                            .filter(|d| pkg == d.name())
                            .count() > 0 {
                        Some(f)
                    } else {
                        None
                    }  
                });

                if let None = package {
                    continue;
                }

                return package
            }
        }

        None
    }
}

pub fn erroneous_transaction<'a>(error: (CommitResult<'a>, alpm::Error)) -> Result<()> {
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

            Err(ErrorKind::TransactionFailure("Conflict within container filesystem".into()))?
        },
        CommitResult::PkgInvalid(p) => {
            for pkg in p.iter() {
                let pkg = format!("{}{pkg}{}", *BOLD_WHITE, *RESET);
                print_error(format!("Invalid package: {}", pkg)); 
            }
        },
        _ => ()
    }

    Err(ErrorKind::TransactionFailure(error.1.to_string()))
}

pub fn erroneous_preparation<'a>(error:  (PrepareResult<'a>, alpm::Error)) -> Result<()> {  
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
        
    Err(ErrorKind::PreparationFailure(error.1.to_string()))
}
