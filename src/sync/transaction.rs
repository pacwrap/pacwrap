use std::collections::HashSet;
use std::rc::Rc;

use bitflags::bitflags;
use alpm::{Alpm, PackageReason};

use crate::config;
use crate::constants::{RESET, BOLD, ARROW_CYAN, BAR_CYAN, ARROW_RED};
use crate::sync::{
    resolver_local::LocalDependencyResolver,
    resolver::DependencyResolver,
    utils::get_local_package,
    utils::get_package};
use crate::utils::print_error;
use crate::config::InstanceHandle;
use self::stage::Stage;
use self::{
    commit::Commit,
    prepare::Prepare, 
    uptodate::UpToDate};

pub use self::aggregator::TransactionAggregator;

pub mod aggregator;
mod commit;
mod prepare;
mod uptodate;
mod stage;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    NothingToDo,
    RecursionDepthExceeded(isize),
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
    Upgrade(bool, bool, bool),
    Remove(bool, bool, bool),
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

bitflags! {
    pub struct TransactionFlags: u8 {
        const NONE = 0;
        const PREVIEW = 0b000001;
        const NO_CONFIRM =  0b000010;
        const FORCE_DATABASE = 0b000100;
        const DATABASE_ONLY = 0b001000;
        const CREATE = 0b010000;
        const FILESYSTEM_SYNC =  0b100000;
    }
}

pub struct TransactionHandle {
    ignore: HashSet<Rc<str>>, 
    ignore_dep: HashSet<Rc<str>>,
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

    fn as_str(&self) -> &str {
        match self {
            Self::Commit(_) => "resident",
            Self::CommitForeign => "foreign",
            _ => ""
        }
    }
}

impl TransactionType { 
    fn as_str(&self) -> &str {
        match self {
            Self::Upgrade(_,_,_) => "installation",
            Self::Remove(_,_,_) => "removal"
        }
    }

    fn action_message(&self, state: TransactionMode) {
        let message = match self {
            Self::Upgrade(_,_,_) => match state {
                TransactionMode::Foreign => "Synchronizing foreign database...",
                TransactionMode::Local => "Synchronizing resident container..."
            }, 
            Self::Remove(_,_,_) => "Preparing package removal..."
        };

        println!("{} {}", *ARROW_CYAN, message);
    }

    fn begin_message(&self, inshandle: &InstanceHandle) {
        let instance = inshandle.vars().instance();
        let message = match self {
            Self::Upgrade(upgrade,_,_) => match upgrade { 
                true => format!("Checking {instance} for updates..."),
                false => format!("Transacting {instance}...")
            }
            Self::Remove(_,_,_) => format!("Transacting {instance}...")
        };

        println!("{} {}{message}{}", *BAR_CYAN, *BOLD, *RESET);
    }
}

impl Error {
    fn message(&self) {
       print_error(match self {
            Self::RecursionDepthExceeded(u) => format!("Recursion depth exceeded maximum of {}{u}{}.", *BOLD, *RESET),
            Self::TargetUpstream(pkg) => format!("Target package {}{pkg}{}: Installed in upstream container.", *BOLD, *RESET),
            Self::TargetNotInstalled(pkg) => format!("Target package {}{pkg}{}: Not installed.", *BOLD, *RESET),
            Self::TargetNotAvailable(pkg) => format!("Target package {}{pkg}{}: Not available in sync databases.", *BOLD, *RESET),
            Self::InitializationFailure(msg) => format!("Failure to initialize transaction: {msg}"),
            Self::PreparationFailure(msg) => format!("Failure to prepare transaction: {msg}"),
            Self::TransactionFailure(msg) => format!("Failure to commit transaction: {msg}"),
            Self::NothingToDo => format!("Nothing to do."),
        });
    }
}

impl TransactionHandle { 
    pub fn new(al: Alpm, q: Vec<Rc<str>>) -> Self {
        Self {
            ignore: HashSet::new(),
            ignore_dep: HashSet::new(),
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
            if let Some(_) = ignored.get(pkg.name().into()) {
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

    fn prepare_add(&mut self, flags: &TransactionFlags) -> Result<()> {
        let ignored = match self.mode { 
            TransactionMode::Foreign => &self.ignore_dep,
            TransactionMode::Local => &self.ignore,
        };

        for queue in self.queue.iter() { 
            if let None = get_package(&self.alpm, queue.as_ref()) { 
                Err(Error::TargetNotAvailable(Rc::clone(queue)))?
            }

            if ignored.contains(queue) && ! self.mode.bool() {
                if flags.contains(TransactionFlags::FORCE_DATABASE) {
                    continue;
                }
            
                Err(Error::TargetUpstream(Rc::clone(queue)))?
            } 
        }        

        let ignored = ignored.iter()
            .map(|i| i.as_ref())
            .collect::<HashSet<_>>();
        let queued = self.queue.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>();
        let packages = DependencyResolver::new(&self.alpm, &ignored).enumerate(&queued)?;

        if packages.0.len() > 0 {
            self.deps = Some(packages.0);
        }

        for pkg in packages.1 {
            if let None = self.ignore.get(pkg.name().into()) {
                if let TransactionMode::Foreign = self.mode {
                    continue;
                }
            }

            self.alpm.trans_add_pkg(pkg).unwrap();        
        }

        Ok(())
    }

    fn prepare_removal(&mut self, enumerate: bool, cascade: bool, explicit: bool) -> Result<()> {
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
            .collect::<HashSet<_>>();
        let queued = self.queue.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>(); 

        for pkg in LocalDependencyResolver::new(&self.alpm, &ignored, enumerate, cascade, explicit).enumerate(&queued)? { 
            self.alpm.trans_remove_pkg(pkg).unwrap(); 
        }

        Ok(())
    }

    fn apply_configuration(&self, instance: &InstanceHandle, create: bool) {
        let depends = instance.metadata().dependencies();
        let pkgs = self.alpm.localdb()
            .pkgs()
            .iter()
            .filter_map(|p| {
                if p.reason() == PackageReason::Explicit
                && ! p.name().starts_with("pacwrap-")
                && ! self.ignore.contains(p.name()) {
                    Some(p.name().into())
                } else {
                    None
                }
            })
        .collect::<Vec<_>>();

        if &pkgs != instance.metadata().explicit_packages() || create {
            let mut instance = instance.clone();
            let depends = depends.clone();

            instance.metadata_mut().set(depends, pkgs);
            config::save_handle(&instance).ok();  
            drop(instance);
        }
    }

    fn trans_ready(&self, trans_type: &TransactionType) -> Result<()> {
        if match trans_type {
            TransactionType::Upgrade(_,_,_) => self.alpm.trans_add().len(),
            TransactionType::Remove(_,_,_) => self.alpm.trans_remove().len()
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
        println!("{} Transaction failed.", *ARROW_RED);
        drop(self);
    }

    fn release(self) {
        drop(self);
    }
    
    fn set_mode(&mut self, modeset: TransactionMode) { 
        self.mode = modeset; 
    }

    fn get_mode(&self) -> &TransactionMode { 
        &self.mode 
    }
    
    fn alpm_mut(&mut self) -> &mut Alpm { 
        &mut self.alpm
    }
    
    fn alpm(&mut self) -> &Alpm { 
        &self.alpm
    }
}
