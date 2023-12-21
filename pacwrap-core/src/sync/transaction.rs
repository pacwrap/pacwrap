use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use bitflags::bitflags;
use alpm::{Alpm, PackageReason, TransFlag};
use serde::{Deserialize, Serialize};

use crate::{config, ErrorKind};
use crate::constants::{RESET, BOLD, ARROW_CYAN, BAR_CYAN, ARROW_RED};
use crate::sync::{
    resolver_local::LocalDependencyResolver,
    resolver::DependencyResolver,
    utils::AlpmUtils};
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
    AgentError,
    NothingToDo,
    DependentContainerMissing(String),
    RecursionDepthExceeded(isize),
    TargetUpstream(String),
    TargetNotInstalled(String),
    TargetNotAvailable(String),
    PreparationFailure(String),
    TransactionFailure(String),
    InitializationFailure(String),
    InternalError(String),
}

pub enum TransactionState {
    Complete(bool),
    Prepare,
    UpToDate,
    PrepareForeign,
    Stage,
    StageForeign,
    Commit(bool),
    CommitForeign,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum TransactionType {
    Upgrade(bool, bool, bool),
    Remove(bool, bool, bool),
}

#[derive(Serialize, Deserialize, Copy, Clone)]
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
        const TARGET_ONLY = 0b0000001; 
        const PREVIEW = 0b0000010;
        const NO_CONFIRM =  0b0000100;
        const FORCE_DATABASE = 0b0001000;
        const DATABASE_ONLY = 0b0010000;
        const CREATE = 0b0100000;
        const FILESYSTEM_SYNC =  0b1000000;
    }
}

pub struct TransactionHandle<'a> {
    meta: &'a mut TransactionMetadata<'a>,
    alpm: Option<Alpm>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionMetadata<'a> {
    foreign_pkgs: HashSet<String>, 
    resident_pkgs: HashSet<String>,
    deps: Option<Vec<String>>,
    queue: Vec<Cow<'a, str>>,
    mode: TransactionMode,
    flags: (u8, u32)
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
            Self::Complete(_) => unreachable!(),
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
    pub fn pr_offset(&self) -> usize {
        match self {
            Self::Upgrade(_,_,_) => 1,
            Self::Remove(_,_,_) => 0
        }
    }

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

impl Display for Error {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> { 
       match self {
            Self::DependentContainerMissing(u) => write!(fmter, "Dependent container '{}{u}{}' is misconfigured or otherwise is missing.", *BOLD, *RESET), 
            Self::RecursionDepthExceeded(u) => write!(fmter, "Recursion depth exceeded maximum of {}{u}{}.", *BOLD, *RESET),
            Self::TargetNotInstalled(pkg) => write!(fmter, "Target package {}{pkg}{}: Not installed.", *BOLD, *RESET), 
            Self::TargetNotAvailable(pkg) => write!(fmter, "Target package {}{pkg}{}: Not available in sync databases.", *BOLD, *RESET),   
            Self::TargetUpstream(pkg) => write!(fmter, "Target package {}{pkg}{}: Installed in upstream container.", *BOLD, *RESET), 
            Self::InitializationFailure(msg) => write!(fmter, "Failure to initialize transaction: {msg}"),
            Self::PreparationFailure(msg) => write!(fmter, "Failure to prepare transaction: {msg}"),
            Self::TransactionFailure(msg) => write!(fmter, "Failure to commit transaction: {msg}"),
            Self::InternalError(msg) => write!(fmter, "Internal failure: {msg}"),
            _ => write!(fmter, "Nothing to do."),
        }
    }
}

impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.into()
    }
}

impl From<ErrorKind> for Error {
    fn from(error: ErrorKind) -> Self {
        Self::InternalError(error.into())
    }
}

impl <'a>TransactionMetadata<'a> {
    fn new(queue: Vec<&'a str>) -> TransactionMetadata {
        Self { 
            foreign_pkgs: HashSet::new(),
            resident_pkgs: HashSet::new(),
            deps: None,
            mode: TransactionMode::Local,
            queue: queue.iter().map(|q| (*q).into()).collect::<Vec<_>>(),
            flags: (0, 0),
        }
    } 
}

impl <'a>TransactionHandle<'a> { 
    pub fn new(alpm_handle: Alpm, metadata: &'a mut TransactionMetadata<'a>) -> Self {
        Self {
            meta: metadata,
            alpm: Some(alpm_handle),
        }  
    }

    fn is_sync_req(&self, mode: TransactionMode) -> SyncReqResult {
        let alpm = self.alpm();
        let ignored = match mode { 
            TransactionMode::Foreign => &self.meta.resident_pkgs,
            TransactionMode::Local => &self.meta.foreign_pkgs,
        };

        for pkg in alpm.localdb().pkgs() {            
            if let Some(_) = ignored.get(pkg.name().into()) {
                continue;
            }

            if let Some(_) = pkg.sync_new_version(alpm.syncdbs()) { 
                return SyncReqResult::Required
            }             
        }

        SyncReqResult::NotRequired
    }

    fn enumerate_foreign_pkgs(&mut self, dep_handle: &Alpm) {
        self.meta.foreign_pkgs.extend(dep_handle.localdb()
            .pkgs()
            .iter()
            .map(|p| p.name().into())
            .filter(|p| ! self.meta.foreign_pkgs.contains(p))
            .collect::<Vec<_>>());
        self.meta.resident_pkgs.extend(self.alpm()
            .localdb()
            .pkgs()
            .iter()
            .map(|a| a.name().into())
            .filter(|p| ! self.meta.foreign_pkgs.contains(p) 
                && ! self.meta.resident_pkgs.contains(p))
            .collect::<Vec<_>>());
    }

    pub fn ignore(&mut self) {
        let ignore = match self.meta.mode { 
            TransactionMode::Foreign => &self.meta.resident_pkgs,
            TransactionMode::Local => &self.meta.foreign_pkgs,
        };
        let unignore = match self.meta.mode { 
            TransactionMode::Local => &self.meta.resident_pkgs,
            TransactionMode::Foreign => &self.meta.foreign_pkgs,
        };
        let alpm = self.alpm.as_mut().unwrap();
  
        for pkg in unignore {
            alpm.remove_ignorepkg(pkg.as_bytes()).unwrap();
        }

        for pkg in ignore {
            alpm.add_ignorepkg(pkg.as_bytes()).unwrap();
        }    
    }

    pub fn prepare(&mut self, trans_type: &TransactionType, flags: &TransactionFlags) -> Result<()> {
        let alpm = self.alpm.as_mut().unwrap();
        let ignored = match self.meta.mode { 
            TransactionMode::Foreign => &self.meta.resident_pkgs,
            TransactionMode::Local => &self.meta.foreign_pkgs,
        };
        let queue = self.meta.queue.iter()
            .map(|i| i.as_ref())
            .collect::<Vec<_>>(); 

        if let TransactionMode::Local = self.meta.mode {
            let upstream = queue.iter()
                .map(|a| *a) 
                .filter(|a| ignored.contains(*a))
                .collect::<Vec<&str>>();

            if ! flags.contains(TransactionFlags::FORCE_DATABASE) {
                if ! upstream.is_empty() {
                    Err(Error::TargetUpstream(upstream[0].to_string()))?
                }
            }
        }
        
        match trans_type {
            TransactionType::Remove(_,_,_) => { 
                let not_installed = queue.iter()
                    .map(|a| *a)  
                    .filter(|a| alpm.get_local_package(a).is_none())
                    .collect::<Vec<&str>>();

                if ! not_installed.is_empty() {
                    Err(Error::TargetNotInstalled(not_installed[0].to_string()))?
                }

                for pkg in LocalDependencyResolver::new(alpm, &ignored, trans_type).enumerate(&queue)? {     
                    alpm.trans_remove_pkg(pkg).unwrap(); 
                }
            },
            TransactionType::Upgrade(_,_,_) => { 
                let not_available = queue.iter()
                    .map(|a| *a)
                    .filter(|a| alpm.get_package(a).is_none()) 
                    .collect::<Vec<&str>>();

                if ! not_available.is_empty() {
                    Err(Error::TargetNotAvailable(not_available[0].to_string()))?
                }

                let packages = DependencyResolver::new(alpm, &ignored).enumerate(&queue)?;

                for pkg in packages.1 {
                    if let None = self.meta.foreign_pkgs.get(pkg.name()) {
                        if let TransactionMode::Foreign = self.meta.mode {
                            continue;
                        }
                    }

                    alpm.trans_add_pkg(pkg).unwrap();        
                }

                self.meta.deps = packages.0;
            }
        }

        Ok(())
    }

    fn apply_configuration(&mut self, instance: &InstanceHandle, create: bool) {
        let depends = instance.metadata().dependencies();
        let pkgs = self.alpm
            .as_mut()
            .unwrap()
            .localdb()
            .pkgs()
            .iter()
            .filter(|p| p.reason() == PackageReason::Explicit
                && ! p.name().starts_with("pacwrap-")
                && ! self.meta.foreign_pkgs.contains(p.name()))
            .map(|p| p.name().into())
        .collect::<Vec<_>>();

        if &pkgs != instance.metadata().explicit_packages() || create {
            let mut instance = instance.clone();
            let depends = depends.clone();

            instance.metadata_mut().set(depends, pkgs);
            config::save_handle(&instance).ok();  
            drop(instance);
        }
    }

    pub fn trans_ready(&mut self, trans_type: &TransactionType) -> Result<()> { 
        if match trans_type {
            TransactionType::Upgrade(_,_,_) => self.alpm().trans_add().len(),
            TransactionType::Remove(_,_,_) => self.alpm().trans_remove().len()
        } > 0 {
            Ok(())
        } else {
            Err(Error::NothingToDo)
        }
    }

    pub fn mark_depends(&mut self) {
        if let Some(deps) = self.meta.deps.as_ref() {
            for mut pkg in deps.iter().filter_map(|a| self.alpm().get_local_package(a)) {
                pkg.set_reason(PackageReason::Depend).unwrap();
            }
        }
    }

    fn release_on_fail(self, error: Error) {
        match error {
            Error::AgentError => (), _ => print_error(error),
        }

        println!("{} Transaction failed.", *ARROW_RED);
        drop(self);
    }

    pub fn release(self) {
        drop(self);
    }
    
    fn set_mode(&mut self, modeset: TransactionMode) { 
        self.meta.mode = modeset; 
    }

    pub fn get_mode(&self) -> &TransactionMode { 
        &self.meta.mode 
    }
    
    pub fn alpm_mut(&mut self) -> &mut Alpm { 
        self.alpm.as_mut().unwrap()
    }
    
    pub fn alpm(&self) -> &Alpm { 
        self.alpm.as_ref().unwrap()
    }

    pub fn set_alpm(&mut self, alpm: Option<Alpm>) {
        self.alpm = alpm;
    }

    pub fn set_flags(&mut self, flags: &TransactionFlags, flags_alpm: TransFlag) {
        self.meta.flags = (flags.bits(), flags_alpm.bits()); 
    }

    pub fn retrieve_flags(&self) -> (Option<TransactionFlags>, Option<TransFlag>) {
        (TransactionFlags::from_bits(self.meta.flags.0), TransFlag::from_bits(self.meta.flags.1))
    }

    fn metadata(&self) -> &TransactionMetadata {
        &self.meta
    }
}
