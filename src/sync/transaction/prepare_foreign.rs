use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator};

pub struct PrepareForeign;

impl Transaction for PrepareForeign { 
    fn new(_: TransactionState) -> Box<Self> {
        Box::new(Self {})
    }

    fn engage(&mut self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> TransactionState {
        let config = inshandle.instance();

        if config.dependencies().len() == 0 {
            return TransactionState::Commit(ag.is_database_only());
        
        }
        if ! ag.is_database_force() { 
            if let Err(_) = handle.out_of_date(true) { 
                if ag.updated()
                    .iter()
                    .filter(|a| config.dependencies()
                        .contains(a)).collect::<Vec<_>>().len() > 0 {
                    return TransactionState::CommitForeign;
                }

                return TransactionState::Commit(ag.is_database_only());
            }
        }

        ag.link_filesystem(inshandle);
        TransactionState::CommitForeign
    }
}
