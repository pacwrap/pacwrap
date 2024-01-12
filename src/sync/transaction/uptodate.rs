use crate::{config::InstanceHandle, constants::ARROW_GREEN};
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator, 
    Result};

pub struct UpToDate;

impl Transaction for UpToDate {
    fn new(_: TransactionState, _: &TransactionAggregator) -> Box<Self> { 
        Box::new(Self {}) 
    }

    fn engage(&self, _: &mut TransactionAggregator, _: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        println!("{} {instance} is up-to-date!", *ARROW_GREEN); 
        Ok(TransactionState::Complete)
    }
}
