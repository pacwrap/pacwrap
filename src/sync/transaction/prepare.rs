use crate::{config::InstanceHandle, sync};
use super::{Transaction, 
    TransactionType, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator};

pub struct Prepare;

impl Transaction for Prepare { 
    fn new(_: TransactionState) -> Box<Self> {
        Box::new(Self {})
    }

    fn engage(&mut self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> TransactionState {
        let deps = inshandle.metadata().dependencies();
        let dep_depth = deps.len(); 
       
        if dep_depth > 0 {
            for dep in deps.iter().rev() {
                let dep_instance = ag.cache().instances().get(dep).unwrap();
                let dep_alpm = sync::instantiate_alpm(dep_instance);
                handle.enumerate_ignorelist(&dep_alpm);
                dep_alpm.release().unwrap();
            }
        }

        if let TransactionType::Upgrade(upgrade) = ag.action() {
            if ! upgrade && handle.queue.len() == 0 {
                return TransactionState::Complete(Err(format!("Nothing to do.")));
            }
        } else {
            if handle.queue.len() == 0 {
                return TransactionState::Complete(Err(format!("Nothing to do.")));
            }  
        }

        if handle.queue.len() == 0 {
            if let Err(_) = handle.out_of_date(false) { 
               return TransactionState::UpToDate; 
            }
        }

        if let TransactionType::Remove(_, _) = ag.action() {
            TransactionState::Commit(ag.is_database_only())
        } else {
            TransactionState::PrepareForeign 
        } 
    }
}
