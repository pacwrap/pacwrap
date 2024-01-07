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

use std::{path::Path,
    fs::{OpenOptions, File}, 
    io::Write, fmt::{Display, Formatter}};

use time::{OffsetDateTime, 
    format_description::FormatItem,
    macros::format_description, UtcOffset};

use crate::{err, 
    impl_error, 
    Error, 
    ErrorKind, 
    ErrorTrait, 
    Result, 
    constants::LOG_LOCATION};

const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour][offset_minute]");
const UTC_OFFSET: &[FormatItem<'static>] = format_description!("[offset_hour]");

impl_error!(LoggerError);

#[derive(Debug)]
pub enum LoggerError {
    Uninitialized,
}

impl Display for LoggerError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::Uninitialized => write!(fmter, "Logger is uninitialized"),
        }
    }
}

pub struct Logger {
    file: Option<File>,
    module: &'static str,
    offset: UtcOffset,
}

impl Logger {
    pub fn new(module_name: &'static str) -> Self { 
       /*
        * In order to deal with the potentiality of a race condition occurring 
        * between libalpm and the time crate, we cache the offset during the 
        * initalisation of this struct.
        */ 
        let ofs = OffsetDateTime::now_local()
            .unwrap_or(OffsetDateTime::now_utc())
            .format(UTC_OFFSET)
            .unwrap();
        let ofs = UtcOffset::parse(ofs.as_str(), UTC_OFFSET).unwrap();

        Self { 
            file:  None,
            module: module_name,
            offset: ofs,
        }
    }

    pub fn init(mut self) -> Result<Self> {
        let path = Path::new(*LOG_LOCATION);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .truncate(false)
            .open(path);

        self.file = Some(match file {
            Ok(file) => file,
            Err(error) => err!(ErrorKind::IOError(LOG_LOCATION.to_string(), error.kind()))?, 
        });

        Ok(self)
    }

    pub fn log(&mut self, msg: impl Into<String> + std::fmt::Display) -> Result<()> { 
       /*
        * We then attempt to update it here.
        *
        * If that fails, we use the previously cached value. This compromise ensures
        * a stale offset value will eventually be updated to reflect the system's 
        * time offset if a change were to occur whilst this application is running.
        */
        if let Ok(local) = OffsetDateTime::now_local() {
            self.offset = UtcOffset::parse(local.format(UTC_OFFSET).unwrap().as_str(), UTC_OFFSET).unwrap();
        }

        let time: OffsetDateTime = OffsetDateTime::now_utc().to_offset(self.offset);
        let write = match self.file.as_mut() {
            Some(file) => file.write(format!("[{}] [{}] {}\n", time.format(DATE_FORMAT).unwrap(), self.module, msg).as_bytes()),
            None => err!(LoggerError::Uninitialized)?
        };

        match write {
            Ok(_) => Ok(()),
            Err(error) => err!(ErrorKind::IOError(LOG_LOCATION.to_string(), error.kind())),
        }
    }
}
