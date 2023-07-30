use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator};

pub struct StateFailure {
    state: TransactionState
}

impl Transaction for StateFailure { 
    fn new(new: TransactionState) -> Box<Self> {
        Box::new(Self { 
            state: new 
        })
    }

    fn engage(&mut self, _: &mut TransactionAggregator, _: &mut TransactionHandle, _: &InstanceHandle) -> TransactionState {
        TransactionState::Complete(Err(format!("Unhandled state failure: {:?} ", self.state)))
    }
}


