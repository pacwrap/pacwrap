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

use nix::sys::termios::{tcgetattr, tcsetattr, SetArg::TCSANOW, Termios};

use crate::{err, Error, ErrorKind, Result};

/*******
 *
 * ermControl struct
 *
 * Impelments basic, portable functionalily for controlling terminal parameters.
 *
 ***/

#[derive(Clone)]
pub struct TermControl {
    tm: Option<Termios>,
    fd: i32,
}

impl TermControl {
    /*
     * A valid termios struct is presumed to be returned
     * if there is a valid tty at specified fd.
     *
     * If the application is not being instantiated from a tty,
     * then return TermControl with tm set with None.
     */

    pub fn new(f: i32) -> Self {
        match tcgetattr(f) {
            Ok(t) => Self { tm: Some(t), fd: f },
            Err(_) => Self { tm: None, fd: f },
        }
    }

    /*
     * Check if Termios initiated and then execute tcsetattr to reset terminal.
     */

    pub fn reset_terminal(&self) -> Result<()> {
        match self.tm.as_ref() {
            Some(tm) => match tcsetattr(self.fd, TCSANOW, tm) {
                Ok(_) => Ok(()),
                Err(errno) => err!(ErrorKind::Termios(errno)),
            },
            None => Ok(()),
        }
    }
}
