use console::style;
use alpm::{Alpm, 
    TransFlag, 
    PrepareResult, 
    CommitResult, 
    FileConflictType};

use crate::{sync, utils::print_error};
use crate::sync::{dl_event, linker};
use crate::sync::dl_event::DownloadCallback;
use crate::sync::progress_event::{self, ProgressCallback};
use crate::sync::linker::Linker;
use crate::sync::query_event::{self, QueryCallback};
use crate::utils::prompt::prompt;
use crate::config::cache::InstanceCache;
use crate::config::InstanceHandle;

pub struct Update {
    queried: Vec<String>,
    updated: Vec<String>,
    linker: Linker
}

impl Update { 
    pub fn new() -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            linker: Linker::new()
        }  
    }

    pub fn linker(&mut self) -> &mut Linker {
        &mut self.linker
    }

    pub fn updated(&self) -> &Vec<String> {
        &self.updated
    }

    pub fn update(&mut self, cache: &InstanceCache, containers: &Vec<String>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let inshandle = cache.instances().get(ins).unwrap();
            
            self.queried.push(ins.clone());
            self.update(cache, inshandle.instance().dependencies());
            self.update_instance(sync::instantiate_alpm(&inshandle), cache, inshandle, false)
                .release()
                .unwrap();
        }
    }

    fn update_instance(&mut self, mut handle: Alpm, cache: &InstanceCache, inshandle: &InstanceHandle, dbonly: bool) -> Alpm {
        let mut flags = TransFlag::NO_DEP_VERSION; 
        let mut ignored: Vec<String> = Vec::new();
        let config = inshandle.instance();
        let deps = config.dependencies();
        let dep_depth = deps.len(); 
        let instance = inshandle.vars().instance(); 

        if ! dbonly {
            println!("{} {}",style("::").bold().cyan(), style(format!("Checking {} for updates...", inshandle.vars().instance())).bold());   
        } else {
            println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing foreign packages...")));    
            flags = flags | TransFlag::DB_ONLY;
        }

        if dep_depth > 0 {
            let dep_handle = sync::instantiate_alpm(cache.instances().get(&deps[dep_depth-1]).unwrap());
            ignored = enumerate_ignorelist(&handle, &dep_handle, dbonly);
        } else if dbonly {
            return handle;
        }

        if let Err(_) = out_of_date(&handle, &ignored) {
            if ! dbonly {
                println!("{} {} is up-to-date!", style("->").bold().green(), instance); 
            }
            return handle;
        }

        if ! dbonly && self.updated
            .iter()
            .find(|ins| inshandle
                .instance()
                .dependencies()
                .contains(ins))
                .is_some() {
                self.link_filesystem(instance, cache);
                handle = self.update_instance(handle, cache, inshandle, true);
        }

        handle.trans_init(flags).unwrap();
        handle.sync_sysupgrade(false).unwrap();
        sync_new_packages(&handle, &ignored);

        if ! dbonly {
            if confirm_transaction(&handle).is_err() {
                handle.trans_release().unwrap();
                return handle;
            }
            
            handle.set_question_cb(QueryCallback, query_event::questioncb);
            handle.set_progress_cb(ProgressCallback::new(true), progress_event::progress_event);
       } 

        if let Err(e) = handle.trans_prepare() {
            handle_erroneous_preparation(e.0, e.1); 
        }

        if let Err(e) = handle.trans_commit() {
            handle_erroneous_transaction(e.0, e.1);
        }

        self.updated.push(instance.clone()); 
        handle.trans_release().unwrap();
        handle
    }

    fn link_filesystem(&mut self, ins: &String, cache: &InstanceCache) {
        println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing container filesystem...")));     
        linker::wait_on(self.linker.link(cache, &vec![ins.clone()], Vec::new()));
    }
}

pub fn update(mut update: Update, cache: &InstanceCache) {
    update.update(&cache, &cache.containers_base());
    update.update(&cache, &cache.containers_dep());
       
    if update.updated().len() > 0 {
        println!("{} {} ",style("::").bold().green(), style("Synchronising container filesystems...").bold()); 
        update.linker().start(cache.registered().len());
        linker::wait_on(update.linker().link(&cache, cache.registered(), Vec::new()));
        update.linker().finish();
    }

    update.update(&cache, &cache.containers_root());
    println!("{} Transaction complete.",style("->").bold().green());
}

fn confirm_transaction(handle: &Alpm) -> Result<(),()> {
    println!("{} {} \n",style("::").bold().red(), style("Package changes").bold());

    let mut installed_size_old: i64 = 0;
    let mut installed_size: i64 = 0;
    let mut download: i64 = 0;
    let mut files_to_download: usize = 0;

    for val in handle.trans_add() { 
        let pkg_sync = val;
        let pkg;

        if let Ok(p) = handle.localdb().pkg(pkg_sync.name()) {
            pkg = p;
        } else {
            pkg = pkg_sync;
        }

        installed_size_old += pkg.isize();             
        installed_size += pkg_sync.isize();
        download += pkg_sync.download_size();

        if download > 0 {
            files_to_download += 1;
        }

        println!("{} {} -> {}", pkg.name(), style(pkg.version()).bold().yellow(), style(pkg_sync.version()).bold().green());
    }
              
    let net = installed_size-installed_size_old;

    println!("\n{}: {}", style("Total Installed Size").bold(), format_unit(installed_size));  
    println!("{}: {}", style("Net Upgrade Size").bold(), format_unit(net)); 
               
    if download > 0 {
        println!("{}: {}", style("Total Download Size").bold(), format_unit(download));
        handle.set_dl_cb(DownloadCallback::new(download.try_into().unwrap(), files_to_download), dl_event::download_event);
 
    }

    println!();
    prompt("::", style("Proceed with installation?").bold(), true)
}

fn handle_erroneous_transaction<'a>(result: CommitResult<'a>, error: alpm::Error) {
    match result {
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
            let mut pkg_string = String::new(); 
            for pkg in p.iter() {
                let pkg = style(pkg).bold().white();  
                pkg_string.push_str(format!("{}, ", pkg).as_str());
            }
            pkg_string.truncate(pkg_string.len()-2);

            print_error(format!("Invalid packages found: {}", pkg_string));
        },
        CommitResult::Ok => print_error(format!("{}", error)) //haha, this should **never** happen
    }

    println!("{} Transaction failed.", style("->").red());
    std::process::exit(1);
}

fn handle_erroneous_preparation<'a>(result: PrepareResult<'a>, error: alpm::Error) {
    match result {
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
        PrepareResult::Ok => print_error(format!("{}", error))
    }

    println!("{} Transaction failed.", style("->").red());
    std::process::exit(1);
}

fn unit_suffix<'a>(i: i8) -> &'a str {
    match i {
        0 => "KB",
        1 => "MB",
        2 => "GB",
        3 => "TB",
        4 => "PB",
        _ => "B"
    }
}

fn format_unit(bytes: i64) -> String {
    let conditional: f64 = if bytes > -1 { 1000.0 } else { -1000.0 };
    let diviser: f64 = 1000.0;
    let mut size: f64 = bytes as f64;
    let mut idx: i8 = -1;

    while if bytes > -1 { size > conditional } else { size < conditional } {
        size = size / diviser;
        idx += 1;
    }
    
    if idx == -1 {
        format!("{:.0} {}", size, unit_suffix(idx))
    } else {
        format!("{:.2} {}", size, unit_suffix(idx)) 
    }
}

fn enumerate_ignorelist(handle: &Alpm, dep_handle: &Alpm, dbonly: bool) -> Vec<String> {
    let mut ignored = Vec::new();

    if dbonly {
        for pkg in handle.localdb().pkgs() { 
            if let Ok(_) = dep_handle.localdb().pkg(pkg.name())  {
                continue; 
            }
            ignored.push(pkg.name().into()); 
        }
    } else {
        for pkg in dep_handle.localdb().pkgs() {
            ignored.push(pkg.name().into()); 
        }
    }

    ignored
}

fn out_of_date(handle: &Alpm, ignored: &Vec<String>) -> Result<(), ()> {
    for pkg in handle.localdb().pkgs() {            
        if ignored.contains(&pkg.name().into()) {
            continue;
        }

        if pkg.sync_new_version(handle.syncdbs()).is_some() { 
            return Ok(());
        }             
    }

    Err(())
}

fn sync_new_packages(handle: &Alpm, ignored: &Vec<String>) {
    for pkg in handle.localdb().pkgs() {
        if ignored.contains(&pkg.name().into()) {
            continue;
        }

        if let Some(pkg) = pkg.sync_new_version(handle.syncdbs()) {
            handle.trans_add_pkg(pkg).unwrap();
        }             
    }
}
