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

use std::{os::unix::process::ExitStatusExt, process::Child};

use crate::{
    config::{
        global::{global, Global},
        ContainerHandle,
    },
    constants::{BOLD, RESET},
    err,
    exec::transaction_agent,
    log::Level::Info,
    sync::{
        self,
        event::summary::Summary,
        transaction::{
            SyncState,
            Transaction,
            TransactionAggregator,
            TransactionFlags,
            TransactionHandle,
            TransactionMode,
            TransactionParameters,
            TransactionState::{self, *},
            TransactionType::{self, *},
        },
        utils::erroneous_preparation,
        SyncError,
    },
    utils::prompt::prompt,
    Error,
    ErrorGeneric,
    Result,
};

enum State {
    Commit((u64, u64)),
    Next(TransactionState),
}

#[derive(Debug)]
pub struct Commit {
    state: TransactionState,
    keyring: bool,
}

impl Transaction for Commit {
    fn new(new: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        let kr = if let Commit(kr) = new { kr } else { false };

        Box::new(Self { state: new, keyring: kr })
    }

    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        handle: &mut TransactionHandle,
        inshandle: &ContainerHandle,
    ) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        let state = self.state.as_str();

        if let SyncState::NotRequired = handle.trans_ready(ag.action(), ag.flags())? {
            handle.alpm_mut().trans_release().generic()?;

            return Ok(match ready_state(ag.action(), &self.state) {
                Some(state) => state,
                None => TransactionState::Complete(false),
            });
        }

        if let Err(error) = handle.alpm_mut().trans_prepare() {
            erroneous_preparation(error)?
        }

        let trans_state = match confirm(&self.state, ag, handle, global()?)? {
            State::Next(state) => return Ok(state),
            State::Commit(values) => values,
        };
        let params = TransactionParameters::new(*ag.action(), *handle.get_mode(), trans_state);

        handle.set_alpm(None);
        ag.lock()?.assert()?;
        wait_on_agent(transaction_agent(inshandle, ag.flags(), params, handle.meta)?)?;

        if self.keyring {
            ag.keyring_update(inshandle)?;
        }

        handle.set_alpm(Some(sync::instantiate_alpm(inshandle, ag.flags())?));
        handle.apply_configuration(inshandle, ag.flags().intersects(TransactionFlags::CREATE))?;
        ag.logger().log(Info, &format!("container {instance}'s {state} transaction complete"))?;
        Ok(next_state(ag.action(), &self.state, true))
    }

    fn debug(&self) -> String {
        format!("{self:?}")
    }
}

fn confirm(
    state: &TransactionState,
    ag: &TransactionAggregator,
    handle: &mut TransactionHandle,
    global: &'static Global,
) -> Result<State> {
    let database = ag.flags().intersects(TransactionFlags::DATABASE_ONLY | TransactionFlags::FORCE_DATABASE);
    let foreign = !handle.get_mode().bool();
    let create = match handle.get_mode() {
        TransactionMode::Foreign => ag.flags().intersects(TransactionFlags::CREATE),
        TransactionMode::Local => false,
    };
    let confirm = foreign || database && !create;
    let sum = Summary::new()
        .kind(global.config().summary(), confirm)
        .mode(handle.get_mode())
        .generate(handle.alpm());

    if confirm {
        println!("{}", sum);

        if ag.flags().contains(TransactionFlags::PREVIEW) {
            handle.alpm_mut().trans_release().generic()?;
            return Ok(State::Next(next_state(ag.action(), state, false)));
        }

        if !ag.flags().contains(TransactionFlags::NO_CONFIRM) {
            let action = ag.action().as_str();
            let query = format!("Proceed with {action}?");

            if !prompt("::", format!("{}{query}{}", *BOLD, *RESET), true)? {
                handle.alpm_mut().trans_release().generic()?;
                return Ok(State::Next(next_state(ag.action(), state, false)));
            }
        }
    }

    handle.alpm_mut().trans_release().generic()?;
    Ok(State::Commit(sum.download()))
}

fn next_state(action: &TransactionType, state: &TransactionState, updated: bool) -> TransactionState {
    match action {
        Remove(..) => match state {
            CommitForeign => Complete(updated),
            Commit(_) => PrepareForeign(updated),
            _ => unreachable!(),
        },
        Upgrade(..) => match state {
            Commit(_) => Complete(updated),
            CommitForeign => Stage,
            _ => unreachable!(),
        },
    }
}

fn ready_state(action: &TransactionType, state: &TransactionState) -> Option<TransactionState> {
    match action {
        Remove(..) => match state {
            CommitForeign => None,
            Commit(_) => Some(PrepareForeign(false)),
            _ => unreachable!(),
        },
        Upgrade(..) => match state {
            Commit(_) => None,
            CommitForeign => Some(Stage),
            _ => unreachable!(),
        },
    }
}

fn wait_on_agent(mut agent: Child) -> Result<()> {
    match agent.wait() {
        Ok(status) => match status.code().unwrap_or(-1) {
            0 => Ok(()),
            1 => err!(SyncError::TransactionAgentError),
            2 | 101 => err!(SyncError::TransactionAgentFailure),
            3 => err!(SyncError::ParameterAcquisitionFailure),
            4 => err!(SyncError::DeserializationFailure),
            5 => err!(SyncError::InvalidMagicNumber),
            6 => err!(SyncError::AgentVersionMismatch),
            _ =>
                if let Some(code) = status.code() {
                    err!(SyncError::TransactionFailure(format!("General agent fault: Exit code {}", code)))
                } else if status.signal().is_some() {
                    err!(SyncError::TransactionFailure(format!("Agent terminated with {}", status)))
                } else {
                    err!(SyncError::TransactionFailure("General agent fault".to_string()))
                },
        },
        Err(error) => err!(SyncError::TransactionFailure(format!("Execution of agent failed: {}", error)))?,
    }
}
