use std::collections::HashMap;
use std::process::exit;

use console::{style, Term};
use alpm::{Alpm, 
    TransFlag, 
    PrepareResult, 
    CommitResult, 
    FileConflictType, Package, PackageReason};

use crate::{sync, utils::print_error};
use crate::sync::{dl_event, linker};
use crate::sync::dl_event::DownloadCallback;
use crate::sync::progress_event::{self, ProgressCallback};
use crate::sync::linker::Linker;
use crate::sync::query_event::{self, QueryCallback};
use crate::utils::prompt::prompt;
use crate::config::cache::InstanceCache;
use crate::config::InstanceHandle;


pub enum TransactionType {
    Upgrade(bool),
    Remove(bool),
}

pub struct TransactionAggregator<'a> {
    queried: Vec<String>,
    updated: Vec<String>,
    pkg_queue: HashMap<String, Vec<String>>,
    action: TransactionType,
    linker: Linker,
    database_only: bool,
    preview: bool,
    no_confirm: bool,
    cache: &'a InstanceCache,
}

impl <'a>TransactionAggregator<'a> { 
    pub fn new(t: TransactionType, icache: &'a InstanceCache) -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            pkg_queue: HashMap::new(),
            linker: Linker::new(),
            action: t, 
            preview: false,
            database_only: false,
            cache: icache,
            no_confirm: false,
        }  
    }

    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    pub fn no_confirm(mut self, no_confirm: bool) -> Self {
        self.no_confirm = no_confirm;
        self
    }

    pub fn database_only(mut self, database_only: bool) -> Self {
        self.database_only = database_only;
        self
    }

    pub fn linker(&mut self) -> &mut Linker {
        &mut self.linker
    }

    pub fn updated(&self) -> &Vec<String> {
        &self.updated
    }

    pub fn queue(&mut self, ins: String, install: Vec<String>) {
        self.pkg_queue.insert(ins, install);
    }

    pub fn transaction(&mut self, containers: &Vec<String>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let cache = self.cache;
            let inshandle = cache.instances().get(ins).unwrap();
        
            self.transaction(inshandle.instance().dependencies());
            self.queried.push(ins.clone());
            self.transact(inshandle);
        }
    }

    pub fn transact(&mut self, inshandle: &InstanceHandle) { 
        let queue = match self.pkg_queue.get(inshandle.vars().instance()) {
            Some(some) => some.clone(), None => Vec::new(),
        };
        let alpm = sync::instantiate_alpm(&inshandle);
        let mut handle = TransactionHandle::new(alpm, queue);
        let mut act: Transaction = Transaction::new(inshandle, &mut handle);
           
        loop {  
            let result = act.engage(self);
               
            if let TransactionState::Complete(result) = result {
                match result {
                    Ok(_) => handle.release(),
                    Err(error) => handle.release_on_fail(error),
                }
                break;
            }
               
            act.next(result);
        }
    }

    fn link_filesystem(&mut self, inshandle: &InstanceHandle) { 
        if inshandle.instance().container_type() == "ROOT" {
            return;
        }

        println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing container filesystem...")));     
        linker::wait_on(self.linker.link(self.cache, &vec![inshandle.vars().instance().into()], Vec::new()));
    }
}

enum TransactionState {
    Complete(Result<(),String>),
    Prepare,
    UpToDate,
    PrepareForeignSync,
    Commit(bool),
    CommitForeignSync,
}

pub struct Transaction<'a> {
    inshandle: &'a InstanceHandle,
    handle: &'a mut TransactionHandle,
    state: TransactionState
}

impl <'a>Transaction<'a> { 
    pub fn new(ins: &'a InstanceHandle, than: &'a mut TransactionHandle) -> Self {
        Self {
            handle: than, 
            inshandle: ins,
            state: TransactionState::Prepare
        }  
    }

    fn next(&mut self, t: TransactionState) {       
        self.state = t; 
    }

    fn engage(&mut self, ag: &mut TransactionAggregator) -> TransactionState {
        match self.state {
            TransactionState::Prepare => self.prepare(ag),
            TransactionState::UpToDate => self.up_to_date(),
            TransactionState::PrepareForeignSync => self.prepare_foreign_sync(ag),
            TransactionState::CommitForeignSync => self.commit_foreign_sync(ag),
            TransactionState::Commit(db) => self.commit(ag,db),
            _ => return TransactionState::Complete(Err("Invalid state.".into()))
        }
    }

    fn up_to_date(&self) -> TransactionState {
        let instance = self.inshandle.vars().instance();
        println!("{} {} is up-to-date!", style("->").bold().green(), instance); 
        TransactionState::Complete(Ok(()))    
    }

    fn commit_foreign_sync(&mut self, ag: &mut TransactionAggregator) -> TransactionState {
        ag.link_filesystem(self.inshandle);
        println!("{} Synchronizing foreign database...",style("->").bold().cyan());        
        
        let result = self.commit(ag,true);
                
        if let TransactionState::Complete(res) = result {
             if let Err(res) = res {
                return TransactionState::Complete(Err(res));
            }
        }
        
        self.handle.db(false);
        TransactionState::Commit(ag.database_only)
    }

    fn prepare_foreign_sync(&mut self, ag: &mut TransactionAggregator) -> TransactionState { 
        let config = self.inshandle.instance();
        
        if config.dependencies().len() > 0 {
            self.handle.db(true);
        } else {
            return TransactionState::Commit(ag.database_only);
        }

        if let Err(_) = self.handle.out_of_date() {
            self.handle.db(false);
            return TransactionState::Commit(ag.database_only);
        }

        TransactionState::CommitForeignSync
    }

    fn prepare(&mut self, ag: &mut TransactionAggregator) -> TransactionState {
        if let TransactionType::Upgrade(_) = ag.action {
            let instance = self.inshandle.vars().instance();  
            println!("{} {}", style("::").bold().cyan(), style(format!("Checking {} for updates...", instance)).bold()); 
        }

        let deps = self.inshandle.instance().dependencies();
        let dep_depth = deps.len(); 
       
        if dep_depth > 0 {
            let dep_instance = ag.cache.instances().get(&deps[dep_depth-1]).unwrap();
            let dep_alpm = sync::instantiate_alpm(dep_instance);
            self.handle.enumerate_ignorelist(&dep_alpm); 
        }

        if let Err(_) = self.handle.out_of_date() {
            if self.handle.queue.len() == 0 {
                return TransactionState::UpToDate;
            }
        }

        TransactionState::PrepareForeignSync
    }

    fn commit(&mut self, ag: &mut TransactionAggregator, dbonly: bool) -> TransactionState { 
        let instance = self.inshandle.vars().instance();
        let flags = match dbonly { 
            false => TransFlag::NO_DEP_VERSION,
            true => TransFlag::NO_DEP_VERSION | TransFlag::DB_ONLY
        };

        let mut set_depends: Vec<String> = Vec::new();

        self.handle.alpm().trans_init(flags).unwrap();

        match ag.action {
            TransactionType::Upgrade(sync) => {
                if sync {
                    self.handle.ignore();
                    self.handle.alpm().sync_sysupgrade(false).unwrap();
                    self.handle.sync();
                }

                match self.handle.prepare_add() {
                    Err(error) => return TransactionState::Complete(Err(error)),
                    Ok(vec) => set_depends = vec,
                }

                if ! self.handle.trans_ready(false) {
                    return TransactionState::Complete(Err("Nothing to do.".into()));
                }

                if ! dbonly || ag.database_only {
                    transaction_summary(self.handle.alpm());

                    if ag.preview {
                        return TransactionState::Complete(Ok(())); 
                    } 

                    if ! ag.no_confirm {
                        if let Err(_) = prompt("::", format!("{}", style("Proceed with installation?").bold()), true) {
                            self.handle.alpm_mut().trans_release().unwrap();
                            return TransactionState::Complete(Ok(()));
                        }
                    }
                }
            },
            TransactionType::Remove(depends) => {
                if let Err(error) = self.handle.prepare_removal(depends) { 
                    return TransactionState::Complete(Err(error));
                }            

                if ! self.handle.trans_ready(true) {
                   return TransactionState::Complete(Err("Nothing to do.".into()));
                }
                
                transaction_summary(self.handle.alpm());

                if ! dbonly {
                    if ag.preview {
                        return TransactionState::Complete(Ok(())); 
                    } 
                   
                    if ! ag.no_confirm {
                        if let Err(_) = prompt("::", format!("{}", style("Proceed with removal?").bold()), true) { 
                            self.handle.alpm_mut().trans_release().unwrap();
                            return TransactionState::Complete(Ok(()));
                        }
                    }
                }
            },
        }
        
        if ! dbonly || ag.database_only { 
            self.handle.alpm().set_question_cb(QueryCallback, query_event::questioncb);
            self.handle.alpm().set_progress_cb(ProgressCallback::new(true), progress_event::progress_event);
        }

        if let Err(_) = handle_preparation(self.handle.alpm_mut().trans_prepare()) { 
            return TransactionState::Complete(Err("Preparation failed.".into()));
        }

        if let Err(_) = handle_transaction(self.handle.alpm_mut().trans_commit()) {
            return TransactionState::Complete(Err("Failed to commit transaction.".into()));
        }

        for pkg in set_depends {
            if let Some(mut pkg) = get_local_package(self.handle.alpm(), pkg.as_str()) {
                pkg.set_reason(PackageReason::Depend).unwrap();
            }
        }

        ag.updated.push(instance.clone()); 
        self.handle.alpm_mut().trans_release().unwrap();

        TransactionState::Complete(Ok(()))
    }
}

pub struct TransactionHandle {
    ignore: Vec<String>, 
    ignore_dep: Vec<String>,
    queue: Vec<String>,
    queue_db: Vec<String>,
    dbonly: bool,
    alpm: Alpm,
    mark_depends: Vec<String>
}

impl TransactionHandle { 
    pub fn new(al: Alpm, q: Vec<String>) -> Self {
        Self {
            ignore: Vec::new(),
            ignore_dep: Vec::new(),
            dbonly: false,
            queue: q,
            queue_db: Vec::new(),
            alpm: al,
            mark_depends: Vec::new()
        }  
    }

    fn out_of_date(&mut self) -> Result<(), ()> {
        let ignored = if self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };

        for pkg in self.alpm.localdb().pkgs() {            
            if ignored.contains(&pkg.name().into()) {
                continue;
            }

            if let Some(_) = pkg.sync_new_version(self.alpm.syncdbs()) { 
                return Ok(())
            }             
        }

        Err(())
    }

    fn enumerate_ignorelist(&mut self, dep_handle: &Alpm) {
        for pkg in self.alpm.localdb().pkgs() { 
            if let Ok(_) = dep_handle.localdb().pkg(pkg.name())  {
                 continue; 
            }
            self.ignore_dep.push(pkg.name().into());  
        }
        
        for pkg in dep_handle.localdb().pkgs() {
            self.ignore.push(pkg.name().into());
        }
    }

    fn ignore(&mut self) {
        let ignore = if self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };        
        let unignore = if ! self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };        
  
        for pkg in unignore {
            self.alpm.remove_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for pkg in ignore {
            self.alpm.add_ignorepkg(pkg.as_bytes()).unwrap();
        }    
    }

    fn prepare_add(&mut self) -> Result<Vec<String>,String> {
        let ignored = if self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };
        let queued = if self.dbonly { 
            &self.queue_db
        } else {
            &self.queue
        };

        for queue in queued {
            if let None = get_package(&self.alpm, queue.as_str()) {
                Err(format!("Target package not found: {}", queue))?
            }
        }

        let ignored = ignored.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let queued = queued.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let packages = DependencyResolver::new(&self.alpm, &ignored).enumerate(&queued);

        for pkg in packages.0 {
            if ignored.contains(&pkg.name()) {
                continue;
            }

            self.alpm.trans_add_pkg(pkg).unwrap();
        }

        Ok(packages.1)
    }

    fn prepare_removal(&mut self, enumerate: bool) -> Result<(),String> {
        let ignor = if self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };

        let ignored = ignor.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let queued = self.queue.iter().map(|i| i.as_str()) .collect::<Vec<_>>(); 

        for queue in queued.iter() {
            if let Some(pkg) = get_local_package(&self.alpm, queue) {
                if ignored.contains(&pkg.name()) {
                    continue;
                }
            
                self.alpm.trans_remove_pkg(pkg).unwrap();
            } else {
                Err(format!("Target package not found: {}", queue))?
            }
        }

        if enumerate { //TODO: Properly complete reverse dependency resolution.
            let packages = InverseDependencyResolver::new(&self.alpm, &ignored).enumerate(&queued);

            for pkg in packages { 
                if let Ok(pkg) = self.alpm.localdb().pkg(pkg.name()) {
                    if ignored.contains(&pkg.name()) {
                        continue;
                    }

                    self.alpm.trans_remove_pkg(pkg).unwrap();
                } 
            }
        }
        Ok(())
    }

    fn trans_ready(&self, remove: bool) -> bool {
        if remove {
            self.alpm.trans_remove().len() > 0
        } else {
            self.alpm.trans_add().len() > 0
        }
    }

    fn sync(&mut self) { 
        let queued = if self.dbonly { 
            &mut self.queue_db
        } else {
            &mut self.queue
        };

        for pkg in self.alpm.trans_add() {
            let deps = pkg.depends().iter().map(|p| p.name()).collect::<Vec<&str>>();
        
            for dep in deps {
                if let None = get_local_package(&self.alpm, dep) { 
                    queued.push(dep.into());
                }
            }   
        }
    }

    fn release_on_fail(mut self, error: String) {
        if error.len() > 0 {
            print_error(error);
        }

        println!("{} Transaction failed.",style("->").bold().red());
        self.alpm.trans_release().ok();
        self.alpm.release().ok(); 
        exit(1);
    }

    fn release(mut self) {
        self.alpm.trans_release().ok();
        self.alpm.release().unwrap();
    }
    
    fn db(&mut self, dbonly: bool) { self.dbonly = dbonly; }
    fn alpm_mut(&mut self) -> &mut Alpm { &mut self.alpm }
    fn alpm(&mut self) -> &Alpm { &self.alpm }
}

struct InverseDependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: i8,
} 

impl <'a>InverseDependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a Vec<&'a str>) -> Self {
        Self {
            resolved: Vec::new(),
            packages: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
        }
    }

    fn check_depth(&mut self) {
        if self.depth == 15 {
            print_error("Recursion depth exceeded maximum.");
            exit(2);
        }
    }
    
    fn enumerate(mut self, packages: &Vec<&'a str>) -> Vec<Package<'a>> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        self.check_depth();

        for pkg in packages {
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            }

            if let Some(pkg) = get_package(&self.handle, pkg) {
                self.resolved.push(pkg.name());
                self.packages.push(pkg);
                let deps = pkg.depends().iter().map(|p| p.name()).collect::<Vec<&str>>(); 

                for dep in deps {
                    if let Some(_) = get_local_package(&self.handle, dep) { 
                        synchronize.push(dep);
                    }
                }
            }             
        }

        if synchronize.len() > 0 {
            self.depth += 1;       
            self.enumerate(&synchronize)
        } else {
            self.packages
        }
    }
}

struct DependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    dependencies: Vec<String>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: i8,
} 

impl <'a>DependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a Vec<&'a str>) -> Self {
        Self {
            resolved: Vec::new(),
            packages: Vec::new(),
            dependencies: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
        }
    }

    fn check_depth(&mut self) {
        if self.depth == 15 {
            print_error("Recursion depth exceeded maximum.");
            exit(2);
        }
    }
    
    fn enumerate(mut self, packages: &Vec<&'a str>) -> (Vec<Package<'a>>, Vec<String>) {
        let mut synchronize: Vec<&'a str> = Vec::new();
        self.check_depth();

        for pkg in packages {
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            } 

            if let Some(pkg) = get_package(&self.handle, pkg) {  
                let deps = pkg.depends().iter().map(|p| p.name()).collect::<Vec<&str>>();

                self.resolved.push(pkg.name());
                self.packages.push(pkg);
                
                if self.depth > 0 {
                    self.dependencies.push(pkg.name().into());
                }

                for dep in deps {
                    if let None = get_local_package(&self.handle, dep) { 
                        synchronize.push(dep);
                        self.dependencies.push(dep.into());
                    }
                }
            }             
        }

        if synchronize.len() > 0 {
            self.depth += 1;
            self.enumerate(&synchronize)
        } else {
            (self.packages, self.dependencies)
        }
    }
}

pub fn update(mut update: TransactionAggregator, cache: &InstanceCache) {
    update.transaction(&cache.containers_base());
    update.transaction(&cache.containers_dep());

    if update.updated().len() > 0 {
        let mut cache = InstanceCache::new();
        cache.populate();
        println!("{} {} ",style("::").bold().green(), style("Synchronising container filesystems...").bold()); 
        update.linker().start(cache.registered().len());
        linker::wait_on(update.linker().link(&cache, cache.registered(), Vec::new()));
        update.linker().finish();
    }

    update.transaction(&cache.containers_root());
    println!("{} Transaction complete.",style("->").bold().green());
}

fn transaction_summary(handle: &Alpm) { 
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

fn handle_transaction<'a>(result: Result<(),(CommitResult<'a>, alpm::Error)>) -> Result<(),()> {
    match result {
        Ok(_) => Ok(()),
        Err(result) => { handle_erroneous_transaction(result); Err(()) }
    }
}

fn handle_erroneous_transaction<'a>(result: (CommitResult<'a>, alpm::Error)) {
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
}

fn handle_preparation<'a>(result: Result<(), (PrepareResult<'a>, alpm::Error)>) -> Result<(),()> {
    match result {
        Ok(_) => Ok(()),
        Err(result) => { handle_erroneous_preparation(result); Err(()) }
    }
}
 

fn handle_erroneous_preparation<'a>(result: (PrepareResult<'a>, alpm::Error)) {
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

fn get_local_package<'a>(handle: &'a Alpm, pkg: &'a str) -> Option<Package<'a>> {
    if let Ok(pkg) = handle.localdb().pkg(pkg) {
        return Some(pkg);
    } else {
        for pkgs in handle.localdb().pkgs() {
            let is_present = pkgs.provides().iter().filter(|d| pkg == d.name()).collect::<Vec<_>>().len() > 0;
            if is_present {
                if let Ok(pkg) = handle.localdb().pkg(pkgs.name()) { 
                    return Some(pkg);
                }
            }
        }
    }
    None
}

fn get_package<'a>(handle: &'a Alpm, pkg: &'a str) -> Option<Package<'a>> {
    for sync in handle.syncdbs() {  
        if let Ok(pkg) = sync.pkg(pkg) {
           return Some(pkg);
        } else {
            for pkgs in sync.pkgs() { 
                let is_present = pkgs.provides().iter().filter(|d| pkg == d.name()).collect::<Vec<_>>().len() > 0;
                if is_present {
                    return Some(pkgs);
                }
            }
        }
    }
    None
}
