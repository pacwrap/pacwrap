use std::rc::Rc;

use console::style;
use alpm::{Alpm, PackageReason};

use crate::sync::{
    resolver_local::LocalDependencyResolver,
    resolver::DependencyResolver,
    utils::get_local_package,
    utils::get_package};
use crate::utils::print_error;
use crate::config::{InstanceHandle, 
    cache::InstanceCache};
use self::stage::Stage;
use self::{
    commit::Commit,
    prepare::Prepare, 
    uptodate::UpToDate};

pub use self::aggregator::TransactionAggregator;

mod aggregator;
mod commit;
mod prepare;
mod uptodate;
mod stage;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    NothingToDo,
    TargetUpstream(Rc<str>),
    TargetNotInstalled(Rc<str>),
    TargetNotAvailable(Rc<str>),
    PreparationFailure(String),
    TransactionFailure(String),
    InitializationFailure(String),
}

pub enum TransactionState {
    Complete,
    Prepare,
    UpToDate,
    PrepareForeign,
    Stage,
    StageForeign,
    Commit(bool),
    CommitForeign,
}

pub enum TransactionType {
    Upgrade(bool),
    Remove(bool, bool),
}

#[derive(Copy, Clone)]
pub enum TransactionMode {
    Foreign,
    Local
}

pub enum SyncReqResult {
    Required,
    NotRequired,
}

pub trait Transaction {
    fn new(new: TransactionState, ag: &TransactionAggregator) -> Box<Self> where Self: Sized;
    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState>;
}

pub struct TransactionHandle {
    ignore: Vec<Rc<str>>, 
    ignore_dep: Vec<Rc<str>>,
    deps: Option<Vec<Rc<str>>>,
    queue: Vec<Rc<str>>,
    mode: TransactionMode,
    alpm: Alpm,
}

impl TransactionMode {
    fn bool(&self) -> bool {
        match self {
            Self::Foreign => true,
            Self::Local => false,
        }
    }
}

impl TransactionState {
    fn from(self, ag: &TransactionAggregator) -> Box<dyn Transaction> {
        match self {
            Self::Prepare => Prepare::new(self, ag),
            Self::PrepareForeign => Prepare::new(self, ag), 
            Self::UpToDate => UpToDate::new(self, ag),
            Self::Stage => Stage::new(self, ag),
            Self::StageForeign => Stage::new(self, ag), 
            Self::Commit(_) => Commit::new(self, ag),
            Self::CommitForeign => Commit::new(self, ag),
            Self::Complete => unreachable!(),
        }
    }
}

impl TransactionType {
    fn as_str(&self) -> &str {
        match self {
            Self::Upgrade(_) => "installation",
            Self::Remove(_, _) => "removal"
        }
    }

    fn action_message(&self, state: TransactionMode) {
        let message = match self {
            Self::Upgrade(_) => match state {
                TransactionMode::Foreign => "Synchronizing foreign database...",
                TransactionMode::Local => "Synchronizing resident container..."
            }, 
            Self::Remove(_, _) => "Preparing package removal..."
        };

        println!("{} {}", style("->").bold().cyan(), message);
    }

    fn begin_message(&self, inshandle: &InstanceHandle) {
        let instance = inshandle.vars().instance();
        let message = match self {
            Self::Upgrade(upgrade) => match upgrade { 
                true => format!("Checking {} for updates...", instance),
                false => format!("Transacting {}...", instance)
            }
            Self::Remove(_,_) => format!("Transacting {}...", instance)
        };

        println!("{} {}", style("::").bold().cyan(), style(message).bold());
    }
}

impl Error {
    fn message(&self) {
       print_error(match self {
            Self::TargetUpstream(pkg) => format!("Target package {}: Installed in upstream container.", style(pkg).bold()),
            Self::TargetNotInstalled(pkg) => format!("Target package {}: Not installed.", style(pkg).bold()),
            Self::TargetNotAvailable(pkg) => format!("Target package {}: Not available in remote repositories.", style(pkg).bold()),
            Self::InitializationFailure(msg) => format!("Failure to initialize transaction: {}", msg),
            Self::PreparationFailure(msg) => format!("Failure to prepare transaction: {}", msg),
            Self::TransactionFailure(msg) => format!("Failure to commit transaction: {}", msg),
            Self::NothingToDo => format!("Nothing to do."),
        });
    }
}

impl TransactionHandle { 
    pub fn new(al: Alpm, q: Vec<Rc<str>>) -> Self {
        Self {
            ignore: Vec::new(),
            ignore_dep: Vec::new(),
            deps: None,
            mode: TransactionMode::Local,
            queue: q,
            alpm: al,
        }  
    }

    fn is_sync_req(&mut self, mode: TransactionMode) -> SyncReqResult {
        let ignored = match mode { 
            TransactionMode::Foreign => &self.ignore_dep,
            TransactionMode::Local => &self.ignore,
        };

        for pkg in self.alpm.localdb().pkgs() {            
            if ignored.contains(&pkg.name().into()) {
                continue;
            }

            if let Some(_) = pkg.sync_new_version(self.alpm.syncdbs()) { 
                return SyncReqResult::Required
            }             
        }

        SyncReqResult::NotRequired
    }

    fn enumerate_ignorelist(&mut self, dep_handle: &Alpm) {
        self.ignore.extend(dep_handle.localdb()
            .pkgs()
            .iter()
            .filter_map(|p| {
                let pkg_name = p.name().into();
            
                if ! self.ignore.contains(&pkg_name) {
                    Some(pkg_name)
                } else {
                    None
                }
            })
            .collect::<Vec<Rc<str>>>());
        self.ignore_dep.extend(self.alpm.localdb()
            .pkgs()
            .iter()
            .filter_map(|p| {
                let pkg_name = p.name().into();

                if ! self.ignore.contains(&pkg_name) 
                && ! self.ignore_dep.contains(&pkg_name) {
                    Some(pkg_name)
                } else {
                    None
                }
            })
            .collect::<Vec<Rc<str>>>());
    }

    fn ignore(&mut self) {
        let ignore = match self.mode { 
            TransactionMode::Foreign => &self.ignore_dep,
            TransactionMode::Local => &self.ignore,
        };
        let unignore = match self.mode { 
            TransactionMode::Local => &self.ignore_dep,
            TransactionMode::Foreign => &self.ignore,
        };
  
        for pkg in unignore {
            self.alpm.remove_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for pkg in ignore {
            self.alpm.add_ignorepkg(pkg.as_bytes()).unwrap();
        }    
    }

    fn prepare_add(&mut self) -> Result<()> {
        let ignored = match self.mode { 
            TransactionMode::Foreign => &self.ignore_dep,
            TransactionMode::Local => &self.ignore,
        };

        for queue in self.queue.iter() { 
            if let None = get_package(&self.alpm, queue.as_ref()) { 
                Err(Error::TargetNotAvailable(Rc::clone(queue)))?
            }

            if ignored.contains(queue) && ! self.mode.bool() {
                Err(Error::TargetUpstream(Rc::clone(queue)))? 
            } 
        }        

        let ignored = ignored.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>();
        let queued = self.queue.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>();
        let packages = DependencyResolver::new(&self.alpm, &ignored).enumerate(&queued);

        if packages.0.len() > 0 {
            self.deps = Some(packages.0);
        }

        for pkg in packages.1 {
            if ! self.ignore.contains(&pkg.name().into()) && self.mode.bool() {
                continue;
            }

            self.alpm.trans_add_pkg(pkg).unwrap();        
        }

        Ok(())
    }

    fn prepare_removal(&mut self, enumerate: bool, cascade: bool) -> Result<()> {
        let ignored = match self.mode { 
            TransactionMode::Foreign => &self.ignore_dep,
            TransactionMode::Local => &self.ignore,
        };

        for queue in self.queue.iter() { 
            if let None = get_local_package(&self.alpm, queue.as_ref()) { 
                Err(Error::TargetNotInstalled(Rc::clone(queue)))? 
            }

            if ignored.contains(queue) && ! self.mode.bool() {
                Err(Error::TargetUpstream(Rc::clone(queue)))? 
            } 
        }

        let ignored = ignored.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>();
        let queued = self.queue.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>(); 

        for pkg in LocalDependencyResolver::new(&self.alpm, &ignored, enumerate, cascade).enumerate(&queued) { 
            self.alpm.trans_remove_pkg(pkg).unwrap(); 
        }

        Ok(())
    }

    fn trans_ready(&self, trans_type: &TransactionType) -> Result<()> {
        if match trans_type {
            TransactionType::Upgrade(_) => self.alpm.trans_add().len(),
            TransactionType::Remove(_,_) => self.alpm.trans_remove().len()
        } > 0 {
            Ok(())
        } else {
            Err(Error::NothingToDo)
        }
    }

    fn mark_depends(&mut self) {
        if let Some(deps) = self.deps.as_ref() {
            for pkg in deps {
                if let Some(mut pkg) = get_local_package(&self.alpm, pkg) {
                    pkg.set_reason(PackageReason::Depend).unwrap();
                }
            }
        }
    }

    fn release_on_fail(self, error: Error) {
        error.message();
        println!("{} Transaction failed.",style("->").bold().red());
        drop(self);
    }

    fn release(self) {
        drop(self);
    }
    
    fn set_mode(&mut self, modeset: TransactionMode) { self.mode = modeset; }
    fn get_mode(&self) -> &TransactionMode { &self.mode }
    fn alpm_mut(&mut self) -> &mut Alpm { &mut self.alpm }
    fn alpm(&mut self) -> &Alpm { &self.alpm }
}

pub fn update<'a>(mut update: TransactionAggregator<'a>, cache: &'a InstanceCache, aux_cache: &'a mut InstanceCache) {
    update.transaction(&cache.containers_base());
    update.transaction(&cache.containers_dep());

    if update.updated().len() > 0 {
        aux_cache.populate(); 

        if aux_cache.containers_root().len() > 0 {
            let linker = update.linker().unwrap();

            linker.set_cache(aux_cache);
            linker.prepare(aux_cache.registered().len());
            linker.engage(&aux_cache.registered());
            linker.finish();
        }
        
        update = update.linker_release(); 
    }

    update.transaction(&cache.containers_root());
    println!("{} Transaction complete.",style("->").bold().green());
}
