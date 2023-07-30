use console::style;

use crate::config::InstanceHandle;
use super::{Transaction, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator};

pub struct UpToDate;

impl Transaction for UpToDate {
    fn new(_: TransactionState) -> Box<Self> { 
        Box::new(Self {}) 
    }

    fn engage(&mut self, _: &mut TransactionAggregator, _: &mut TransactionHandle, inshandle: &InstanceHandle) -> TransactionState {
        let instance = inshandle.vars().instance();
        println!("{} {} is up-to-date!", style("->").bold().green(), instance); 
        TransactionState::Complete(Ok(()))
    }
}
