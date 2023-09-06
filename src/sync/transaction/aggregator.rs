use std::collections::HashMap;
use std::process::exit;
use std::rc::Rc;

use crate::log::Logger;
use crate::sync::{self,
    filesystem::FilesystemStateSync};
use crate::config::{InstanceHandle, 
    InstanceType::ROOT,
    cache::InstanceCache};
use super::{
    Transaction,
    TransactionHandle,
    TransactionState,
    TransactionType,
    TransactionFlags};

#[derive(Debug)]
pub enum Error {
    LinkerUninitialised
}

pub struct TransactionAggregator<'a> {
    queried: Vec<Rc<str>>,
    updated: Vec<Rc<str>>,
    pkg_queue: HashMap<Rc<str>, Vec<Rc<str>>>,
    action: TransactionType,
    filesystem_state: Option<FilesystemStateSync<'a>>,
    cache: &'a InstanceCache,
    keyring: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags
}

impl <'a>TransactionAggregator<'a> { 
    pub fn new(t: TransactionType, inscache: &'a InstanceCache, log: &'a mut Logger) -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            pkg_queue: HashMap::new(),
            filesystem_state: Some(FilesystemStateSync::new(inscache)),
            action: t, 
            cache: inscache,
            keyring: false,
            logger: log,
            flags: TransactionFlags::NONE
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

    pub fn sync_filesystem(&mut self, inshandle: &InstanceHandle) { 
        if let ROOT = inshandle.metadata().container_type() {
            return;
        }

        let fs_sync = self.fs_sync().unwrap();

        fs_sync.prepare_single();
        fs_sync.engage(&vec![inshandle.vars().instance().clone()]);
    }

    pub fn cache(&self) -> &'a InstanceCache { &self.cache }
    pub fn action(&self) -> &TransactionType { &self.action }  
    pub fn updated(&self) -> &Vec<Rc<str>> { &self.updated }
    pub fn is_keyring_synced(&self) -> bool { self.keyring }
   
    pub fn flags(&self) -> &TransactionFlags {
        &self.flags
    }

    pub fn logger(&mut self) -> &mut Logger {
        &mut self.logger
    }

    pub fn fs_sync_release(mut self) -> Self {
        if let Some(_) = self.filesystem_state { 
            self.filesystem_state = self.filesystem_state.unwrap().release();
        }

        self
    }

    pub fn fs_sync(&mut self) -> Result<&mut FilesystemStateSync<'a>, Error> { 
        match self.filesystem_state.as_mut() {
            Some(linker) => Ok(linker),
            None => Err(Error::LinkerUninitialised),
        }
    }

    pub fn preview(mut self, preview: bool) -> Self {
        if preview {
            self.flags = self.flags | TransactionFlags::PREVIEW;
        }
        
        self
    }

    pub fn no_confirm(mut self, no_confirm: bool) -> Self {
        if no_confirm {
            self.flags = self.flags | TransactionFlags::NO_CONFIRM; 
        }

        self
    }

    pub fn force_database(mut self, force_database: bool) -> Self {
        if force_database {
            self.flags = self.flags | TransactionFlags::FORCE_DATABASE;
        }

        self
    }

    pub fn database_only(mut self, database_only: bool) -> Self {
        if database_only {
            self.flags = self.flags | TransactionFlags::DATABASE_ONLY;
        }

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
