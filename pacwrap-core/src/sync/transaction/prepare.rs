use crate::{config::InstanceHandle, sync};

use super::{Transaction, 
    TransactionType, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator, 
    TransactionFlags,
    SyncReqResult, TransactionMode, 
    ErrorKind,
    Result};

pub struct Prepare {
    state: TransactionState,
}

impl Transaction for Prepare { 
    fn new(new_state: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        Box::new(Self {
            state: new_state,
        })
    }

    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> {
        match self.state {
            TransactionState::Prepare => {
                let deps: Vec<&str> = inshandle.metadata().dependencies();
       
                if deps.len() > 0 {
                    for dep in deps.iter().rev() {
                        match ag.cache().get_instance(dep) {
                            Some(dep_handle) => {
                                let dep_alpm = sync::instantiate_alpm(dep_handle);                             
                                handle.enumerate_foreign_pkgs(&dep_alpm); 
                                dep_alpm.release().unwrap();
                            },
                            None => Err(ErrorKind::DependentContainerMissing(dep.to_string()))?,
                        }
                    }   
                }

                if let TransactionType::Upgrade(upgrade,_,_) = ag.action() {
                    if ! upgrade && handle.metadata().queue.len() == 0 {
                        Err(ErrorKind::NothingToDo)?
                    }
                } else {
                    if handle.metadata().queue.len() == 0 {
                        Err(ErrorKind::NothingToDo)?
                    }  
                }

                if handle.metadata().queue.len() == 0 {
                    if let SyncReqResult::NotRequired = handle.is_sync_req(TransactionMode::Local) { 
                        return Ok(TransactionState::UpToDate)
                    }
                }

                if let TransactionType::Remove(_, _,_) = ag.action() {
                    Ok(TransactionState::Stage)
                } else if deps.len() == 0 {
                    Ok(TransactionState::Stage)
                } else {
                    Ok(TransactionState::PrepareForeign)    
                }
            },
            TransactionState::PrepareForeign => {
                if ! ag.flags().contains(TransactionFlags::FORCE_DATABASE) { 
                    if let SyncReqResult::NotRequired = handle.is_sync_req(TransactionMode::Foreign) { 
                        if ag.deps_updated(inshandle) {
                            return Ok(TransactionState::StageForeign)
                        }

                        return Ok(TransactionState::Stage)
                    }
                }

                Ok(TransactionState::StageForeign)
            }
            _ => unreachable!()
        }
    }
}
