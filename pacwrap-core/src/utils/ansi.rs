/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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

use nix::unistd::isatty;

use crate::{
    constants::{COLORTERM, TERM},
    lazy_lock,
};

macro_rules! ansi {
    ( $x:ident = ($y:expr, $z: expr); $($t:tt)* ) => {
        lazy_lock!(pub static ref $x: &'static str = IS_COLOR_TERMINAL.then(|| $y).unwrap_or($z););
        ansi!($($t)*);
    };
    ( $x:ident = $y:expr; $($t:tt)* ) => {
        lazy_lock!(pub static ref $x: &'static str = IS_COLOR_TERMINAL.then(|| $y).unwrap_or_default(););
        ansi!($($t)*);
    };
    () => ()
}

lazy_lock! {
    pub static ref IS_COLOR_TERMINAL: bool = is_color_terminal();
    pub static ref IS_TRUECOLOR_TERMINAL: bool = is_truecolor_terminal();
}

ansi! {
    ARROW_RED = ("[1;31m->[0m", "->");
    ARROW_GREEN = ("[1;32m->[0m", "->");
    ARROW_CYAN = ("[1;36m->[0m", "->");
    BAR_RED = ("[2;31m::[0m", "::");
    BAR_GREEN = ("[1;32m::[0m", "::");
    BAR_CYAN = ("[1;36m::[0m", "::");
    CHECKMARK = (" [1;32m✓[0m", " ✓");
    BOLD_RED = "[1;31m";
    BOLD_GREEN = "[1;32m";
    BOLD_YELLOW = "[1;33m";
    BOLD_WHITE = "[1;37m";
    YELLOW = "[33m";
    RESET = "[0m";
    BOLD = "[1m";
    DIM = "[2m";
    UNDERLINE = "[4m";
}

pub fn is_truecolor_terminal() -> bool {
    let value = COLORTERM.to_lowercase();

    is_color_terminal() && value == "truecolor" || value == "24bit"
}

pub fn is_color_terminal() -> bool {
    !TERM.is_empty() && TERM.to_lowercase() != "dumb" && isatty(0).is_ok() && isatty(1).is_ok()
}
