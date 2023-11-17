use std::collections::HashMap;
use std::process::exit;
use std::rc::Rc;

use crate::constants::ARROW_GREEN;
use crate::config::InstanceType;
use crate::exec::utils::execute_in_container;
use crate::log::Logger;
use crate::sync::{self,
    filesystem::FileSystemStateSync};
use crate::config::{InstanceHandle, 
    InstanceType::ROOT,
    cache::InstanceCache};
use super::{
    Transaction,
    TransactionHandle,
    TransactionState,
    TransactionType,
    TransactionFlags, TransactionMetadata};

#[derive(Debug)]
pub enum Error {
    LinkerUninitialised,
}

pub struct TransactionAggregator<'a> {
    queried: Vec<Rc<str>>,
    updated: Vec<Rc<str>>,
    pkg_queue: HashMap<&'a str, Vec<&'a str>>,
    action: TransactionType,
    filesystem_state: Option<FileSystemStateSync<'a>>,
    cache: &'a InstanceCache,
    keyring: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags,
    target: Option<&'a str>,
}

impl <'a>TransactionAggregator<'a> { 
    pub fn new(
        inscache: &'a InstanceCache, 
        queue: HashMap<&'a str, Vec<&'a str>>, 
        log: &'a mut Logger, 
        action_flags: TransactionFlags, 
        action_type: TransactionType,  
        current_target: Option<&'a str>) -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
            pkg_queue: queue,
            filesystem_state: Some(FileSystemStateSync::new(inscache)),
            action: action_type, 
            cache: inscache,
            keyring: false,
            logger: log,
            flags:  action_flags,
            target: current_target,
        }
    }


    pub fn aggregate(mut self, aux_cache: &'a mut InstanceCache) {
        let upgrade = match self.action {
            TransactionType::Upgrade(upgrade, refresh, force) => {
                if refresh {
                    sync::synchronize_database(self.cache, force); 
                }

                upgrade
            },
            _ => false,
        };
        let target = match self.target {
            Some(s) => self.cache.instances().get(s), None => None
        };

        if let Some(inshandle) = target { 
            if let InstanceType::BASE | InstanceType::DEP = inshandle.metadata().container_type() {
                self.transact(inshandle); 
            }
        } else if upgrade {
            self.transaction(self.cache.containers_base());
            self.transaction(self.cache.containers_dep());
        }

        if self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC | TransactionFlags::CREATE) 
        || self.updated.len() > 0 {
            aux_cache.populate(); 

            if aux_cache.containers_root().len() > 0 {
                let linker = self.fs_sync().unwrap();

                linker.set_cache(aux_cache);
                linker.prepare(aux_cache.registered().len());
                linker.engage(&aux_cache.registered());
                linker.finish();
            }
        
            self.filesystem_state = self.filesystem_state.unwrap().release();  
        }

        if let Some(inshandle) = target {
            if let InstanceType::ROOT = inshandle.metadata().container_type() {
                self.transact(inshandle); 
            }
        } else if upgrade {
            self.transaction(self.cache.containers_root());
        }

        println!("{} Transaction complete.", *ARROW_GREEN);
    }

    pub fn transaction(&mut self, containers: &Vec<Rc<str>>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let inshandle = self.cache
                .instances()
                .get(ins);

            if let Some(inshandle) = inshandle {
                self.transaction(inshandle.metadata().dependencies());
                self.queried.push(ins.clone());
                self.transact(inshandle);
            }
        }
    }

    pub fn transact(&mut self, inshandle: &InstanceHandle) { 
        let queue = match self.pkg_queue.get(inshandle.vars().instance().as_ref()) {
            Some(some) => some.clone(), None => Vec::new(),
        };
        let alpm = sync::instantiate_alpm(&inshandle);
        let meta = TransactionMetadata::new(queue);
        let mut handle = TransactionHandle::new(alpm, meta);
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

    pub fn keyring_update(&mut self, inshandle: &InstanceHandle) {
        execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--populate", "archlinux"));
        execute_in_container(inshandle, vec!("/usr/bin/pacman-key", "--updatedb"));
        self.keyring = true;
    }

    pub fn sync_filesystem(&mut self, inshandle: &InstanceHandle) { 
        if let ROOT = inshandle.metadata().container_type() {
            return;
        }

        let fs_sync = self.fs_sync().unwrap();

        fs_sync.prepare_single();
        fs_sync.engage(&vec![inshandle.vars().instance().clone()]);
    }

    pub fn cache(&self) -> &InstanceCache { 
        &self.cache
    }
    
    pub fn action(&self) -> &TransactionType { 
        &self.action 
    }

    pub fn updated(&self) -> &Vec<Rc<str>> { 
        &self.updated 
    }

    pub fn is_keyring_synced(&self) -> bool { 
        self.keyring 
    }
   
    pub fn flags(&self) -> &TransactionFlags {
        &self.flags
    }

    pub fn logger(&mut self) -> &mut Logger {
        &mut self.logger
    }

    pub fn fs_sync(&mut self) -> Result<&mut FileSystemStateSync<'a>, Error> { 
        match self.filesystem_state.as_mut() {
            Some(linker) => Ok(linker),
            None => Err(Error::LinkerUninitialised),
        }
    }

    pub fn set_updated(&mut self, updated: Rc<str>) {
        self.updated.push(updated);
    }
}
