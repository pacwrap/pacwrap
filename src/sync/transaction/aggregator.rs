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
use crate::utils::{Arguments, print_help_error};
use crate::utils::arguments::Operand;
use super::{
    Transaction,
    TransactionHandle,
    TransactionState,
    TransactionType,
    TransactionFlags};

#[derive(Debug)]
pub enum Error {
    LinkerUninitialised,
}

pub struct TransactionAggregator<'a> {
    queried: Vec<Rc<str>>,
    updated: Vec<Rc<str>>,
    pkg_queue: HashMap<Rc<str>, Vec<Rc<str>>>,
    action: TransactionType,
    filesystem_state: Option<FileSystemStateSync<'a>>,
    cache: &'a InstanceCache,
    keyring: bool,
    logger: &'a mut Logger,
    flags: TransactionFlags,
    target: Option<&'a str>,
}

impl <'a>TransactionAggregator<'a> {  
    pub fn aggregate(mut self, aux_cache: &'a mut InstanceCache) {
        let upgrade = if let TransactionType::Upgrade(upgrade, refresh, force) = self.action { 
            if refresh {
                sync::synchronize_database(self.cache, force); 
            }

            upgrade
        } else {
            false
        };

        if let Some(target) = self.target {
            let inshandle = self.cache.instances().get(target).unwrap();

            if let InstanceType::BASE | InstanceType::DEP = inshandle.metadata().container_type() {
                self.transact(inshandle); 
            }
        } else if upgrade {
            self.transaction(self.cache.containers_base());
            self.transaction(self.cache.containers_dep());
        }

        if self.flags.intersects(TransactionFlags::CREATE) || self.flags.intersects(TransactionFlags::FILESYSTEM_SYNC) || self.updated.len() > 0 {
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

        if let Some(target) = self.target {
            let inshandle = self.cache.instances().get(target).unwrap();

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

pub fn remove<'a>(action_type: TransactionType, args: &'a mut Arguments, inscache: &'a mut InstanceCache, log: &'a mut Logger) -> TransactionAggregator<'a> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<Rc<str>,Vec<Rc<str>>> = HashMap::new();
    let mut current_target = "";

    args.set_index(1);

    if let Operand::None = args.next().unwrap_or_default() {
        print_help_error("Operation not specified.");
    }

    while let Some(arg) = args.next() {
        match arg {
            Operand::Long("remove")
                | Operand::Long("cascade") 
                | Operand::Long("recursive") 
                | Operand::Short('R')
                | Operand::Short('c')  
                | Operand::Short('s') 
                | Operand::Short('t') => continue,  
            Operand::Long("noconfirm") | Operand::Long("no-confirm") => action_flags = action_flags | TransactionFlags::NO_CONFIRM,                  
            Operand::Short('p') | Operand::Long("preview") => action_flags = action_flags | TransactionFlags::PREVIEW, 
            Operand::Long("db-only") => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Short('f') | Operand::Long("filesystem") => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) 
                | Operand::ShortPos(_, target) => {
                current_target = target;
                targets.push(target.into());
            },
            Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { queue.insert(current_target.into(), vec!(package.into())); },
                }
            },
            _ => args.invalid_operand(),
        }
    }
        
    if current_target == "" {
        print_help_error("Target not specified");
    }

    let current_target = Some(current_target);

    if targets.len() > 0 {
        inscache.populate_from(&targets, true);
    } else {
        inscache.populate();
    }
 
    TransactionAggregator {
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

pub fn upgrade<'a>(action_type: TransactionType, args: &'a mut Arguments, inscache: &'a mut InstanceCache, log: &'a mut Logger) -> TransactionAggregator<'a> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<Rc<str>,Vec<Rc<str>>> = HashMap::new();
    let mut current_target = "";
    let mut target_only = false;
    let mut base = false;

    args.set_index(2);

    if let Operand::None = args.next().unwrap_or_default() {
        print_help_error("Operation not specified.");
    }

    while let Some(arg) = args.next() {
        match arg {
                Operand::Short('d') | Operand::Long("slice")
                | Operand::Short('r') | Operand::Long("root") 
                | Operand::Short('t') | Operand::Long("target") 
                | Operand::Short('y') | Operand::Long("refresh")
                | Operand::Short('u') | Operand::Long("upgrade") => continue,
            Operand::Short('o') | Operand::Long("target-only") => target_only = true,
            Operand::Short('f') | Operand::Long("filesystem") => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::Short('p') | Operand::Long("preview") => action_flags = action_flags | TransactionFlags::PREVIEW,
            Operand::Short('c') | Operand::Long("create") => action_flags = action_flags | TransactionFlags::CREATE,
            Operand::Short('b') | Operand::Long("base") => base = true,
            Operand::Long("db-only") => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Long("noconfirm") => action_flags = action_flags | TransactionFlags::NO_CONFIRM, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) => {
                current_target = target;
                targets.push(target.into());
            },
            Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { 
                        let packages = if base {
                            base = false;
                            vec!(package.into(), "base".into(), "pacwrap-base-dist".into())
                        } else {
                            vec!(package.into())
                        };

                        queue.insert(current_target.into(), packages); 
                    },
                }
            },
            Operand::None => println!("none"),
            _ => args.invalid_operand(),
        }
    }

    let current_target = match target_only {
        true => {
            if current_target == "" {
                print_help_error("Target not specified");
            }

            Some(current_target)
        },
        false => None,
    };
 
    if targets.len() > 0 {
        inscache.populate_from(&targets, true);
    } else {
        inscache.populate();
    }
 
    TransactionAggregator {
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
