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
use alpm::Alpm;
use dialoguer::console::Term;
use simplebyteunit::simplebyteunit::{SI, ToByteUnit};

use crate::{err,
    Error,
    Result,
    exec::transaction_agent, 
    sync::{self,
        SyncError,
        transaction::{Transaction, 
            TransactionState, 
            TransactionHandle, 
            TransactionAggregator,
            TransactionFlags, 
            TransactionParameters},
        utils::erroneous_preparation}, 
    utils::prompt::prompt,
    constants::{RESET, BOLD, DIM},
    config::InstanceHandle};

pub struct Commit {
    state: TransactionState,
    keyring: bool,
}

impl Transaction for Commit { 
    fn new(new: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        let kr = match new { 
            TransactionState::Commit(bool) => bool, _ => false
        };

        Box::new(Self { 
            state: new,
            keyring: kr,
        })
    }

    fn engage(&self, ag: &mut TransactionAggregator, handle: &mut TransactionHandle, inshandle: &InstanceHandle) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        let ready = handle.trans_ready(&ag.action());
        let state = self.state.as_str();

        if let Err(_) = ready {
            match self.state { 
                TransactionState::Commit(_) => ready?,
                TransactionState::CommitForeign => return state_transition(&self.state, handle, false),
                _ => unreachable!()
            }
        } 

        if let Err(error) = handle.alpm_mut().trans_prepare() {
            erroneous_preparation(error)?
        }

        let result = confirm(&self.state, ag, handle);
        let result = match result.0 {
            Some(result) => return result, None => result.1
        };

        handle.set_alpm(None); 

        let parameters =  &TransactionParameters::new(*ag.action(), *handle.get_mode(), result);
        let mut agent = transaction_agent(inshandle, parameters, handle.metadata())?;

        match agent.wait() {
            Ok(exit_status) => match exit_status.code().unwrap_or(-1) {
                0 => {
                    if self.keyring {
                        ag.keyring_update(inshandle)?;
                    }

                    handle.set_alpm(Some(sync::instantiate_alpm(inshandle))); 
                    handle.apply_configuration(inshandle, ag.flags().intersects(TransactionFlags::CREATE))?; 
                    ag.logger().log(format!("container {instance}'s {state} transaction complete")).ok();
                    state_transition(&self.state, handle, true)
                },
                1 => err!(SyncError::TransactionFailureAgent),
                2 => err!(SyncError::ParameterAcquisitionFailure),
                3 => err!(SyncError::DeserializationFailure), 
                4 => err!(SyncError::InvalidMagicNumber),
                5 => err!(SyncError::AgentVersionMismatch),
                _ => err!(SyncError::TransactionFailure(format!("Generic failure of agent: Exit code {}", exit_status.code().unwrap_or(-1))))?,  
            },
            Err(error) => err!(SyncError::TransactionFailure(format!("Execution of agent failed: {}", error)))?,
        }
    } 
}

fn confirm(state: &TransactionState, ag: &mut TransactionAggregator, handle: &mut TransactionHandle) -> (Option<Result<TransactionState>>, (u64, u64)) {
    let sum = summary(handle.alpm());

    if ! handle.get_mode().bool() || ag.flags().intersects(TransactionFlags::DATABASE_ONLY | TransactionFlags::FORCE_DATABASE) {
        println!("{}", sum.0);

        if ag.flags().contains(TransactionFlags::PREVIEW) {
            return (Some(state_transition(state, handle, false)), sum.1); 
        } 

        if ! ag.flags().contains(TransactionFlags::NO_CONFIRM) {
            let action = ag.action().as_str();
            let query = format!("Proceed with {action}?");

            if let Err(_) = prompt("::", format!("{}{query}{}", *BOLD, *RESET), true) {
                return (Some(state_transition(state, handle, false)), sum.1);
            }
        } 
    }

    handle.alpm_mut().trans_release().ok();
    (None, sum.1)
}

fn state_transition<'a>(state: &TransactionState, handle: &mut TransactionHandle, updated: bool) -> Result<TransactionState> {
    handle.alpm_mut().trans_release().ok();
 
    Ok(match state {
        TransactionState::Commit(_) => TransactionState::Complete(updated),
        TransactionState::CommitForeign => TransactionState::Stage,
        _ => unreachable!()
    })
}

fn summary(handle: &Alpm) -> (String, (u64, u64)) { 
    let mut installed_size: i64 = 0;
    let mut installed_size_old: i64 = 0; 
    let mut download: i64 = 0;
    let mut files_to_download: u64 = 0;
    let mut current_line_len: usize = 0;
    let remove = if handle.trans_remove().len() > 0 { true } else { false };
    let packages = if remove { handle.trans_remove() } else { handle.trans_add() };
    let size = Term::size(&Term::stdout());
    let preface = format!("Packages ({}) ", packages.len());
    let preface_newline = " ".repeat(preface.len()); 
    let line_delimiter = size.1 as usize - preface.len();
    let mut pkglist: String = String::new(); 
    let mut summary = format!("\n{}{preface}{}", *BOLD, *RESET);

    for pkg_sync in packages { 
        let pkg = match handle.localdb().pkg(pkg_sync.name()) {
            Ok(pkg) => pkg, Err(_) => pkg_sync,
        };
        let output = format!("{}-{}{}{} ", pkg.name(), *DIM, pkg_sync.version(), *RESET); 
        let download_size = pkg_sync.download_size();
        let string_len = pkg.name().len() + pkg_sync.version().len() + 2;

        if current_line_len+string_len >= line_delimiter { 
            summary.push_str(&format!("{pkglist}\n"));
            pkglist = preface_newline.clone();
            current_line_len = pkglist.len(); 
        }

        current_line_len += string_len;
        installed_size_old += pkg.isize(); 
        installed_size += pkg_sync.isize();
        
        if download_size > 0 {
            download += download_size;
            files_to_download += 1;
        }

        pkglist.push_str(&output);  
    }

    let total_str = if remove { "Total Removed Size" } else { "Total Installed Size" }; 
    let net = installed_size-installed_size_old;

    summary.push_str(&format!("{pkglist}\n\n{}{total_str}{}: {}\n", *BOLD, *RESET, installed_size.to_byteunit(SI))); 
  
    if net != 0 {
        summary.push_str(&format!("{}Net Upgrade Size{}: {}\n", *BOLD, *RESET, net.to_byteunit(SI))); 
    }

    if download > 0 {
        summary.push_str(&format!("{}Total Download Size{}: {}\n", *BOLD, *RESET, download.to_byteunit(SI)));
    }

    (summary, (download as u64, files_to_download))
}
