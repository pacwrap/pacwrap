/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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
use crate::{
    config::{ContainerHandle, ContainerType},
    constants::UNIX_TIMESTAMP,
    err,
    sync::{
        self,
        schema::{self, *},
        transaction::{
            SyncState::*,
            Transaction,
            TransactionAggregator,
            TransactionFlags,
            TransactionHandle,
            TransactionMode,
            TransactionState::{self, *},
            TransactionType::*,
        },
        SyncError,
    },
    Error,
    Result,
};

#[derive(Debug)]
pub struct Prepare {
    state: TransactionState,
}

impl Transaction for Prepare {
    fn new(new_state: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        Box::new(Self { state: new_state })
    }

    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        handle: &mut TransactionHandle,
        inshandle: &ContainerHandle,
    ) -> Result<TransactionState> {
        match self.state {
            Prepare => {
                let deps: Vec<&str> = inshandle.metadata().dependencies();
                let instype = inshandle.metadata().container_type();
                let action = ag.action();

                if let (ContainerType::Base, false) = (instype, ag.updated(inshandle)) {
                    if let SchemaStatus::OutOfDate(set) = schema::version(inshandle)? {
                        return Ok(UpdateSchema(set));
                    }
                }

                if !deps.is_empty() {
                    for dep in deps.iter().rev() {
                        match ag.cache().get_instance_option(dep) {
                            Some(dep_handle) => handle.enumerate_package_lists(&sync::instantiate_alpm(dep_handle, ag.flags())),
                            None => err!(SyncError::DependentContainerMissing(dep.to_string()))?,
                        }
                    }

                    let create = ag.flags().contains(TransactionFlags::CREATE);
                    let lazy_load = ag.flags().contains(TransactionFlags::LAZY_LOAD_DB);
                    let timestamp = inshandle.metadata().timestamp();
                    let present = *UNIX_TIMESTAMP;

                    if !lazy_load && create && present == timestamp {
                        handle.enumerate_foreign_queue();
                    }
                }

                if let Upgrade(upgrade, ..) = action {
                    if !upgrade && handle.meta.queue.is_empty() {
                        err!(SyncError::NothingToDo)?
                    }
                } else if handle.meta.queue.is_empty() {
                    err!(SyncError::NothingToDo)?
                }

                if handle.meta.queue.is_empty() {
                    if let NotRequired = handle.is_sync_req(TransactionMode::Local) {
                        return Ok(UpToDate);
                    }
                }

                if let Remove(..) = action {
                    Ok(Stage)
                } else if let ContainerType::Base = instype {
                    Ok(Stage)
                } else {
                    Ok(PrepareForeign(false))
                }
            }
            PrepareForeign(updated) => {
                if let ContainerType::Base = inshandle.metadata().container_type() {
                    return Ok(Complete(updated));
                }

                if ag.flags().contains(TransactionFlags::FORCE_DATABASE) {
                    return Ok(StageForeign);
                }

                match ag.action() {
                    Remove(..) => Ok(Complete(updated)),
                    Upgrade(..) => Ok(match handle.is_sync_req(TransactionMode::Foreign) {
                        Required => StageForeign,
                        NotRequired => Stage,
                    }),
                }
            }
            _ => unreachable!(),
        }
    }

    fn debug(&self) -> String {
        format!("{self:?}")
    }
}
