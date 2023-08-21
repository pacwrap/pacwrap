use std::process::exit;

use console::style;
use alpm::Alpm;

use crate::sync::{
    resolver_local::LocalDependencyResolver,
    resolver::DependencyResolver,
    utils::get_local_package,
    utils::get_package};
use crate::utils::{
    print_error, 
    print_warning};
use crate::config::{InstanceHandle, 
    cache::InstanceCache};
use self::{
    commit::Commit,
    prepare_foreign::PrepareForeign, 
    prepare::Prepare, 
    uptodate::UpToDate,
    unhandled::StateFailure};

pub use self::aggregator::TransactionAggregator;

mod aggregator;
mod commit;
mod prepare;
mod prepare_foreign;
mod uptodate;
mod unhandled;

pub trait Transaction {
    fn new(new: TransactionState) -> Box<Self> where Self: Sized;
    fn engage(&mut self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> TransactionState;
}

#[derive(Debug)]
pub enum TransactionState {
    Complete(Result<(),String>),
    Prepare,
    UpToDate,
    PrepareForeign,
    Commit(bool),
    CommitForeign,
}

impl TransactionState {
    fn from(self) -> Box<dyn Transaction> {
        match self {
            Self::Prepare => Prepare::new(self),
            Self::UpToDate => UpToDate::new(self),
            Self::Commit(_) => Commit::new(self),
            Self::CommitForeign => Commit::new(self),
            Self::PrepareForeign => PrepareForeign::new(self), 
            _ => StateFailure::new(self)
        }
    }
}

pub enum TransactionType {
    Upgrade(bool),
    Remove(bool, bool),
}

impl TransactionType {
    fn as_str(&self) -> &str {
        match self {
            Self::Upgrade(_) => "installation",
            Self::Remove(_, _) => "removal"
        }
    }

    fn action_message(&self, dbonly: bool) {
        let message = match self {
            Self::Upgrade(_) => match dbonly {
                true => "Synchronizing foreign database...",
                false => "Synchronizing resident container..."
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

pub struct TransactionHandle {
    ignore: Vec<String>, 
    ignore_dep: Vec<String>,
    queue: Vec<String>,
    dbonly: bool,
    alpm: Alpm,
}

impl TransactionHandle { 
    pub fn new(al: Alpm, q: Vec<String>) -> Self {
        Self {
            ignore: Vec::new(),
            ignore_dep: Vec::new(),
            dbonly: false,
            queue: q,
            alpm: al,
        }  
    }

    fn out_of_date(&mut self, dbonly: bool) -> Result<(), ()> {
        let ignored = if dbonly { 
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
        for pkg in dep_handle.localdb().pkgs() {
            let pkg_name = pkg.name().into();

            if ! self.ignore.contains(&pkg_name) {
                self.ignore.push(pkg_name);
            }
        }

        for pkg in self.alpm.localdb().pkgs() {             
            let pkg_name = pkg.name().into();

            if self.ignore.contains(&pkg_name) {
                continue;
            }

            if ! self.ignore_dep.contains(&pkg_name) { 
                self.ignore_dep.push(pkg_name); 
            }
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

        for queue in self.queue.iter() { 
            if let None = get_package(&self.alpm, queue.as_str()) { 
                Err(format!("Target package {}: Not found.", style(queue).bold()))?
            }

            if ignored.contains(queue) && ! self.dbonly {
                 print_warning(format!("Target package {}: Installed in upstream container.", style(queue).bold()));
            }
        }

        let ignored = ignored.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let queued = self.queue.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let packages = DependencyResolver::new(&self.alpm, &ignored).enumerate(&queued);

        for pkg in packages.0 {
            if ! self.ignore.contains(&pkg.name().into()) && self.dbonly {
                continue;
            }

            self.alpm.trans_add_pkg(pkg).unwrap();        
        }

        Ok(packages.1)
    }

    fn prepare_removal(&mut self, enumerate: bool, cascade: bool) -> Result<(),String> {
        let ignored = if self.dbonly { 
            &self.ignore_dep
        } else {
            &self.ignore
        };

        for queue in self.queue.iter() { 
            if let None = get_local_package(&self.alpm, queue.as_str()) { 
                Err(format!("Target package {}: Not installed.", style(queue).bold()))?
            }

            if ignored.contains(queue) && ! self.dbonly {
                 print_warning(format!("Target package {}: Installed in upstream container.", style(queue).bold()));
            } 
        }

        let ignored = ignored.iter().map(|i| i.as_str()) .collect::<Vec<_>>();
        let queued = self.queue.iter().map(|i| i.as_str()) .collect::<Vec<_>>(); 
        let packages = LocalDependencyResolver::new(&self.alpm, &ignored, enumerate, cascade).enumerate(&queued);

        for pkg in packages { 
            self.alpm.trans_remove_pkg(pkg).unwrap(); 
        }

        Ok(())
    }

    fn trans_ready(&self, trans_type: &TransactionType) -> bool {
        match trans_type {
            TransactionType::Upgrade(_) => self.alpm.trans_add().len() > 0,
            TransactionType::Remove(_,_) => self.alpm.trans_remove().len() > 0
        }
    }

    fn sync(&mut self) { 
        let queued = &mut self.queue;

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

pub fn update(mut update: TransactionAggregator, cache: &InstanceCache) {
    update.transaction(&cache.containers_base());
    update.transaction(&cache.containers_dep());

    update.linker().finish();

    if update.updated().len() > 0 {
        let mut cache = InstanceCache::new();
        println!("{} {} ",style("::").bold().green(), style("Synchronising container filesystems...").bold());  
        cache.populate();
        update.linker().start(cache.registered().len());
        update.linker().link(&cache.registered(), 0);
        update.linker().finish();
    }

    update.transaction(&cache.containers_root());
    println!("{} Transaction complete.",style("->").bold().green());
}
