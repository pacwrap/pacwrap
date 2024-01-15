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

use super::{
    Result,
    Transaction,
    TransactionAggregator,
    TransactionHandle,
    TransactionState::{self, Complete},
};
use crate::{config::InstanceHandle, constants::ARROW_GREEN};

pub struct UpToDate;

impl Transaction for UpToDate {
    fn new(_: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        Box::new(Self {})
    }

    fn engage(
        &self,
        _: &mut TransactionAggregator,
        _: &mut TransactionHandle,
        inshandle: &InstanceHandle,
    ) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        println!("{} {instance} is up-to-date!", *ARROW_GREEN);
        Ok(Complete(false))
    }
}
