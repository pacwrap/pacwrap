use std::collections::HashMap;
use std::process::exit;
use std::rc::Rc;

use crate::sync::{self,
    linker::FilesystemStateSync};
use crate::utils::print_warning;
use crate::config::{InstanceHandle, 
    InstanceType::ROOT,
    cache::InstanceCache};
use super::{
    Transaction,
    TransactionHandle,
    TransactionState,
    TransactionType};

#[derive(Debug)]
pub enum Error {
    LinkerUninitialised
}

pub struct TransactionAggregator<'a> {
    queried: Vec<Rc<str>>,
    updated: Vec<Rc<str>>,
    pkg_queue: HashMap<Rc<str>, Vec<Rc<str>>>,
    action: TransactionType,
    linker: Option<FilesystemStateSync<'a>>,
    force_database: bool,
    database_only: bool,
    preview: bool,
    no_confirm: bool,
    cache: &'a InstanceCache,
    keyring: bool,
}

impl <'a>TransactionAggregator<'a> { 
    pub fn new(t: TransactionType, icache: &'a InstanceCache) -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            pkg_queue: HashMap::new(),
            linker: Some(FilesystemStateSync::new(icache)),
            action: t, 
            force_database: false,
            preview: false,
            database_only: false,
            cache: icache,
            no_confirm: false,
            keyring: false,
        }  
    }

    pub fn transaction(&mut self, containers: &Vec<Rc<str>>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let cache = self.cache;
            let inshandle = cache.instances().get(ins);

            if let Some(inshandle) = inshandle {
                self.transaction(inshandle.metadata().dependencies());
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
        let mut act: Box<dyn Transaction> = TransactionState::Prepare.from(self);
        
        self.action.begin_message(&inshandle);

        loop {  
            let result = match act.engage(self, &mut handle, inshandle) {
                Ok(result) => {
                    if let TransactionState::Complete = result {
                        handle.release();
                        break;
                    }
                    
                    result
                },
                Err(result) => { 
                    handle.release_on_fail(result);
                    exit(1);
                }
            };
               
            act = result.from(self);
        }
    }

    pub fn link_filesystem(&mut self, inshandle: &InstanceHandle) { 
        if let ROOT = inshandle.metadata().container_type() {
            return;
        }

        self.linker().unwrap().prepare_single();
        self.linker().unwrap().engage(&vec![inshandle.vars().instance().clone()]);
    }

    pub fn cache(&self) -> &'a InstanceCache { &self.cache }
    pub fn action(&self) -> &TransactionType { &self.action }  
    pub fn updated(&self) -> &Vec<Rc<str>> { &self.updated }
    pub fn skip_confirm(&self) -> bool { self.no_confirm } 
    pub fn is_preview(&self) -> bool { self.preview } 
    pub fn is_keyring_synced(&self) -> bool { self.keyring }
    pub fn is_database_only(&self) -> bool { self.database_only }
    pub fn is_database_force(&self) -> bool { self.force_database } 
    
    pub fn linker_release(mut self) -> Self {
        if let Some(_) = self.linker { 
            self.linker = self.linker.unwrap().release();
        }

        self
    }

    pub fn linker(&mut self) -> Result<&mut FilesystemStateSync<'a>, Error> { 
        match self.linker.as_mut() {
            Some(linker) => Ok(linker),
            None => Err(Error::LinkerUninitialised),
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

    pub fn force_database(mut self, force_database: bool) -> Self {
        self.force_database = force_database;
        self
    }

    pub fn database_only(mut self, database_only: bool) -> Self {
        self.database_only = database_only;
        self
    }

    pub fn queue(&mut self, ins: Rc<str>, install: Vec<Rc<str>>) {
        self.pkg_queue.insert(ins, install);
    }

    pub fn set_keyring_synced(&mut self) {
        self.keyring = true;
    }

    pub fn set_updated(&mut self, updated: Rc<str>) {
        self.updated.push(updated);
    }
}

