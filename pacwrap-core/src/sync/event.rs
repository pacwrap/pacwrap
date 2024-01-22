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

use crate::utils;

pub mod download;
pub mod progress;
pub mod query;

fn whitespace(total: usize, current: usize) -> String {
    utils::whitespace(log10(total) - log10(current))
}

fn log10(mut value: usize) -> usize {
    let mut length = 0;

    while value > 0 {
        value /= 10;
        length += 1;
    }

    length
}
