use std::collections::HashMap;

use console::style;

use crate::sync::{self,
    linker::{self, Linker}};
use crate::utils:: print_warning;
use crate::config::{InstanceHandle, 
    cache::InstanceCache};
use super::{
    Transaction,
    TransactionHandle,
    TransactionState,
    TransactionType};

pub struct TransactionAggregator<'a> {
    queried: Vec<String>,
    updated: Vec<String>,
    pkg_queue: HashMap<String, Vec<String>>,
    action: TransactionType,
    linker: Linker,
    force_database: bool,
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
            force_database: false,
            preview: false,
            database_only: false,
            cache: icache,
            no_confirm: false,
        }  
    }

    pub fn transaction(&mut self, containers: &Vec<String>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let cache = self.cache;
            let inshandle = cache.instances().get(ins);

            if let Some(inshandle) = inshandle {
                self.transaction(inshandle.instance().dependencies());
                self.queried.push(ins.clone());
                self.transact(inshandle);
            } else {
                print_warning(format!("Handle for {} not initialised.", ins));
            }
        }
    }

    pub fn transact(&mut self, inshandle: &InstanceHandle) { 
        let queue = match self.pkg_queue.get(inshandle.vars().instance()) {
            Some(some) => some.clone(), None => Vec::new(),
        };
        let alpm = sync::instantiate_alpm(&inshandle);
        let mut handle = TransactionHandle::new(alpm, queue);
        let mut act: Box<dyn Transaction> = TransactionState::Prepare.from();
        
        self.action.begin_message(&inshandle);

        loop {  
            let result = act.engage(self, &mut handle, inshandle);

            if let TransactionState::Complete(result) = result {
                match result {
                    Ok(_) => handle.release(),
                    Err(error) => handle.release_on_fail(error),
                }
                break;
            }
               
            act = result.from();
        }
    }

    pub fn link_filesystem(&mut self, inshandle: &InstanceHandle) { 
        if inshandle.instance().container_type() == "ROOT" {
            return;
        }

        println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing container filesystem...")));     
        linker::wait_on(self.linker.link(self.cache, &vec![inshandle.vars().instance().into()], Vec::new()));
    }

    pub fn cache(&self) -> &'a InstanceCache { &self.cache }
    pub fn action(&self) -> &TransactionType { &self.action } 
    pub fn linker(&mut self) -> &mut Linker { &mut self.linker }
    pub fn updated(&self) -> &Vec<String> { &self.updated }
    pub fn skip_confirm(&self) -> bool { self.no_confirm } 
    pub fn is_preview(&self) -> bool { self.preview }
    pub fn is_database_only(&self) -> bool { self.database_only }
    pub fn is_database_force(&self) -> bool { self.force_database } 

    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    pub fn no_confirm(mut self, no_confirm: bool) -> Self {
        self.no_confirm = no_confirm;
        self
    }

    pub fn force_database(mut self, force_database: bool) -> Self {
        self.force_database = force_database;
        self
    }

    pub fn database_only(mut self, database_only: bool) -> Self {
        self.database_only = database_only;
        self
    }

    pub fn queue(&mut self, ins: String, install: Vec<String>) {
        self.pkg_queue.insert(ins, install);
    }

    pub fn set_updated(&mut self, updated: String) {
        self.updated.push(updated);
    }
}

