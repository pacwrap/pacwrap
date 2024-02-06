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

use crate::{
    config::ContainerHandle,
    constants::CHECKMARK,
    log::Level::Info,
    sync::{
        schema::extract,
        transaction::{
            Transaction,
            TransactionAggregator,
            TransactionHandle,
            TransactionState::{self, *},
        },
    },
    Result,
};

pub struct Schema {
    state: TransactionState,
}

impl Transaction for Schema {
    fn new(t_state: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        Box::new(Self { state: t_state })
    }

    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        _: &mut TransactionHandle,
        inshandle: &ContainerHandle,
    ) -> Result<TransactionState> {
        let instance = inshandle.vars().instance();
        let schema = match &self.state {
            UpdateSchema(schema) => schema,
            _ => unreachable!(),
        };

        extract(inshandle, schema)?;
        println!("{} {instance}'s schema updated.", *CHECKMARK);
        ag.logger().log(Info, &format!("container {instance}'s filesystem schema updated.")).ok();
        Ok(TransactionState::Prepare)
    }
}
