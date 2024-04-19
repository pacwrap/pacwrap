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
use alpm::TransFlag;

use crate::{
    config::{ContainerHandle, ContainerType::Base},
    err,
    sync::{
        transaction::{
            Transaction,
            TransactionAggregator,
            TransactionFlags,
            TransactionHandle,
            TransactionMode::{self, *},
            TransactionState::{self, *},
            TransactionType::*,
        },
        SyncError,
    },
    Error,
    Result,
};

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
            modeset = Local;
            flag = TransFlag::NO_DEP_VERSION;

            if ag.flags().contains(TransactionFlags::DATABASE_ONLY) {
                flag = flag | TransFlag::DB_ONLY;
            }
        } else {
            modeset = Foreign;
            flag = TransFlag::NO_DEP_VERSION | TransFlag::DB_ONLY;
        }

        Box::new(Self {
            state: new,
            flags: flag,
            mode: modeset,
        })
    }

    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        handle: &mut TransactionHandle,
        inshandle: &ContainerHandle,
    ) -> Result<TransactionState> {
        if let Err(error) = handle.alpm().trans_init(self.flags) {
            err!(SyncError::InitializationFailure(error.to_string().into()))?
        }

        ag.action().action_message(self.mode);
        handle.set_mode(self.mode);
        handle.ignore();
        handle.meta.set_flags(ag.flags(), &self.flags);

        match ag.action() {
            Upgrade(upgrade, downgrade, _) => {
                if *upgrade {
                    handle.alpm().sync_sysupgrade(*downgrade).unwrap();
                }

                handle.prepare(ag.action(), ag.flags())?;
                next_state(&self.state, check_keyring(ag, handle, inshandle))
            }
            Remove(..) => {
                handle.prepare(ag.action(), ag.flags())?;
                next_state(&self.state, false)
            }
        }
    }
}

fn check_keyring(ag: &TransactionAggregator, handle: &TransactionHandle, inshandle: &ContainerHandle) -> bool {
    inshandle.metadata().container_type() == &Base
        && !ag.is_keyring_synced()
        && handle.alpm().trans_add().iter().find(|a| a.name() == "archlinux-keyring").is_some()
}

fn next_state(state: &TransactionState, option: bool) -> Result<TransactionState> {
    Ok(match state {
        Stage => Commit(option),
        StageForeign => CommitForeign,
        _ => unreachable!(),
    })
}
