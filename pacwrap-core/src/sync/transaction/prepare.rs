/*
 * pacwrap-core
 * 
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use crate::{err, 
    Error, 
    Result, 
    config::InstanceHandle,     
    sync::{self,
        SyncError,
        transaction::{Transaction, 
            TransactionState,
            TransactionMode,
            TransactionType,
            TransactionHandle, 
            TransactionAggregator,
            TransactionFlags,
            SyncReqResult}}};

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
                            None => err!(SyncError::DependentContainerMissing(dep.to_string()))?,
                        }
                    }   
                }

                if let TransactionType::Upgrade(upgrade,_,_) = ag.action() {
                    if ! upgrade && handle.metadata().queue.len() == 0 {
                        err!(SyncError::NothingToDo(true))?
                    }
                } else {
                    if handle.metadata().queue.len() == 0 {
                        err!(SyncError::NothingToDo(true))?
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
