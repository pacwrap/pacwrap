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
    config::ContainerHandle,
    constants::ARROW_GREEN,
    sync::transaction::{
        aggregator::BAR_GREEN_STYLE,
        Transaction,
        TransactionAggregator,
        TransactionHandle,
        TransactionState::{self, Skip},
    },
    Result,
};

pub struct UpToDate;

impl Transaction for UpToDate {
    fn new(_: TransactionState, _: &TransactionAggregator) -> Box<Self> {
        Box::new(Self)
    }

    fn engage(
        &self,
        ag: &mut TransactionAggregator,
        _: &mut TransactionHandle,
        handle: &ContainerHandle,
    ) -> Result<TransactionState> {
        match ag.progress_bar() {
            Some(progress) =>
                if progress.position() == progress.length().unwrap_or(0) {
                    progress.set_style(BAR_GREEN_STYLE.clone());
                    progress.finish();
                },
            None => println!("{} {} is up-to-date!", *ARROW_GREEN, handle.vars().instance()),
        }

        Ok(Skip)
    }
}
