use alpm::TransFlag;

use crate::config::InstanceType;
use crate::config::InstanceHandle;
use super::Error;
use super::Result;
use super::TransactionMode;
use super::{Transaction, 
    TransactionType, 
    TransactionState, 
    TransactionHandle, 
    TransactionAggregator, 
    TransactionFlags};

pub struct Stage {
    state: TransactionState,
    mode: TransactionMode,
    flags: TransFlag,
}

impl Transaction for Stage { 
    fn new(new: TransactionState, ag: &TransactionAggregator) -> Box<Self> { 
        let mut flag;
        let modeset;

        if let TransactionState::Stage = new {
            modeset = TransactionMode::Local;
            flag = TransFlag::NO_DEP_VERSION;
                
            if ag.flags().contains(TransactionFlags::DATABASE_ONLY) {
                flag = flag | TransFlag::DB_ONLY;
            }
        } else {
            modeset = TransactionMode::Foreign;
            flag = TransFlag::NO_DEP_VERSION | TransFlag::DB_ONLY;
        }

        Box::new(Self { 
            state: new,
            flags: flag,
            mode: modeset
        })
    }

    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> { 
        if let Err(error) = handle.alpm().trans_init(self.flags) {
            Err(Error::InitializationFailure(error.to_string().into()))?
        }

        ag.action().action_message(self.mode);
        handle.set_mode(self.mode);
        handle.ignore();

        match ag.action() {
            TransactionType::Upgrade(upgrade, downgrade, _) => {  
                if *upgrade {
                    handle.alpm().sync_sysupgrade(*downgrade).unwrap();
                }

                handle.prepare_add(ag.flags())?;
                state_transition(&self.state, check_keyring(ag, handle, inshandle))
            },
            TransactionType::Remove(depends, cascade, explicit) => {
                handle.prepare_removal(*depends, *cascade, *explicit)?;
                state_transition(&self.state, false)
            },
        }
    }
}

fn check_keyring(ag: &TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> bool {
    match inshandle.metadata().container_type() {
        InstanceType::BASE => {
            if ag.is_keyring_synced() {
                return false
            }
 
            handle.alpm()
                .trans_add()
                .iter()
                .find_map(|a| Some(a.name() == "archlinux-keyring"))
                .unwrap_or(false)
        },
        _ => false
    }
}

fn state_transition(state: &TransactionState, option: bool) -> Result<TransactionState> {
    Ok(match state {
        TransactionState::Stage => TransactionState::Commit(option),
        TransactionState::StageForeign => TransactionState::CommitForeign,
        _ => unreachable!()
    })
}
