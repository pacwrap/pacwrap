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

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use time::{format_description::FormatItem, macros::format_description, OffsetDateTime, UtcOffset};

use crate::{
    constants::{LOG_LOCATION, UNIX_TIMESTAMP},
    err,
    impl_error,
    Error,
    ErrorKind,
    ErrorTrait,
    Result,
};

const DATE_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour][offset_minute]");
const UTC_OFFSET: &[FormatItem<'static>] = format_description!("[offset_hour]");

impl_error!(LoggerError);

#[derive(Debug)]
pub enum LoggerError {
    Uninitialized,
}

impl Display for LoggerError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Uninitialized => write!(fmter, "Logger is uninitialized"),
        }
    }
}

#[derive(PartialEq)]
pub enum Level {
    Info,
    Warn,
    Error,
    Debug,
    Fatal,
}

impl Level {
    fn to_str(&self) -> &str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
            Self::Debug => "DEBUG",
        }
    }

    fn verbosity(&self) -> i8 {
        self.into()
    }
}

impl From<&Level> for i8 {
    fn from(val: &Level) -> Self {
        match val {
            Level::Info => 0,
            Level::Warn => 1,
            Level::Error => 2,
            Level::Fatal => 3,
            Level::Debug => 4,
        }
    }
}

impl From<i8> for Level {
    fn from(i8: i8) -> Self {
        match i8 {
            0 => Self::Info,
            1 => Self::Warn,
            2 => Self::Error,
            3 => Self::Fatal,
            4 => Self::Debug,
            _ => panic!("Invalid i8 input"),
        }
    }
}

impl Display for Level {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmter, "{}", self.to_str())
    }
}

pub struct Logger {
    verbosity: i8,
    file: Option<File>,
    module: &'static str,
    offset: UtcOffset,
}

impl Logger {
    pub fn new(module_name: &'static str) -> Self {
        /*
         * In order to deal with the potentiality of a race condition occurring
         * between libalpm and the time crate, cache the offset during the
         * initalisation of this struct.
         */
        let ofs = OffsetDateTime::now_local()
            .unwrap_or(OffsetDateTime::now_utc())
            .format(UTC_OFFSET)
            .unwrap();
        let ofs = UtcOffset::parse(ofs.as_str(), UTC_OFFSET).unwrap();

        Self {
            verbosity: 3,
            file: None,
            module: module_name,
            offset: ofs,
        }
    }

    pub fn init(mut self) -> Result<Self> {
        let path = Path::new(*LOG_LOCATION);
        let file = OpenOptions::new().create(true).append(true).truncate(false).open(path);

        self.file = Some(match file {
            Ok(file) => file,
            Err(error) => err!(ErrorKind::IOError(LOG_LOCATION.to_string(), error.kind()))?,
        });
        Ok(self)
    }

    pub fn set_verbosity(&mut self, verbosity: i8) {
        self.verbosity = verbosity
    }

    pub fn log(&mut self, level: Level, msg: &str) -> Result<()> {
        if level.verbosity() > self.verbosity {
            return Ok(());
        }

        /*
         * Then attempt to update it here.
         *
         * If that fails, use the previously cached value. This compromise ensures
         * a stale offset value will eventually be updated to reflect the system's
         * time offset if a change were to occur whilst this application is running.
         */
        if let Ok(local) = OffsetDateTime::now_local() {
            self.offset = UtcOffset::parse(local.format(UTC_OFFSET).unwrap().as_str(), UTC_OFFSET).unwrap();
        }

        let time: OffsetDateTime = OffsetDateTime::now_utc().to_offset(self.offset);
        let write = if let Some(file) = self.file.as_mut() {
            file.write(format!("[{}] [{}] [{}] {}\n", time.format(DATE_FORMAT).unwrap(), self.module, level, msg).as_bytes())
        } else {
            err!(LoggerError::Uninitialized)?
        };

        if let Level::Debug = level {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("SystemTime now");
            let time = now.as_secs() as usize - *UNIX_TIMESTAMP as usize;
            let nano = now.subsec_nanos().to_string();

            eprintln!("[{}.{:.6}] [{}] {}", time, nano, self.module, msg);
        }

        match write {
            Ok(_) => Ok(()),
            Err(error) => err!(ErrorKind::IOError(LOG_LOCATION.to_string(), error.kind())),
        }
    }
}
