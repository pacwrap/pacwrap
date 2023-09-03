use crate::{config::InstanceHandle, sync};
use super::{Transaction, 
    TransactionType, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator, 
    TransactionFlags,
    SyncReqResult, TransactionMode, 
    Error,
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
                let deps = inshandle.metadata().dependencies();
       
                if deps.len() > 0 {
                    for dep in deps.iter().rev() {
                        let dep_instance = ag.cache().instances().get(dep).unwrap();
                        let dep_alpm = sync::instantiate_alpm(dep_instance);
                        handle.enumerate_ignorelist(&dep_alpm);
                        dep_alpm.release().unwrap();
                    }
                }

                if let TransactionType::Upgrade(upgrade) = ag.action() {
                    if ! upgrade && handle.queue.len() == 0 {
                        Err(Error::NothingToDo)?
                    }
                } else {
                    if handle.queue.len() == 0 {
                        Err(Error::NothingToDo)?
                    }  
                }

                if handle.queue.len() == 0 {
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
                        if ag.updated()
                            .iter()
                            .filter(|a| inshandle.metadata()
                                .dependencies()
                                .contains(a))
                                .collect::<Vec<_>>().len() > 0 {
                                    return Ok(TransactionState::StageForeign)
                            }

                            return Ok(TransactionState::Stage)
                        }
                    }

                ag.sync_filesystem(inshandle);
                Ok(TransactionState::StageForeign)
            }
            _ => unreachable!()
        }
    }
}
