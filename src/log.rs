use std::{path::Path,
    fs::{OpenOptions, File}, 
    io::{Write, Error}, sync::Arc};

use time::{OffsetDateTime, 
    format_description::FormatItem,
    macros::format_description, UtcOffset};

use crate::constants::LOGGER_LOCATION;

const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour][offset_minute]");
const UTC_OFFSET: &[FormatItem<'static>] = format_description!("[offset_hour]");

#[derive(Debug)]
pub enum LoggerError {
    ParentNotFound,
    Uninitialized,
    GenericIOError(Arc<str>, Error)
}

pub struct Logger {
    file: Option<File>,
    module: Arc<str>,
    offset: UtcOffset,
}

impl Logger {
    pub fn new(module_name: impl Into<Arc<str>>) -> Self { 
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
            module: module_name.into(),
            offset: ofs,
        }
    }

    pub fn init(mut self) -> Result<Self, LoggerError> {
        let path = Path::new(LOGGER_LOCATION.as_ref());
        
        if ! path.parent().unwrap().exists() {
            Err(LoggerError::ParentNotFound)?
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .truncate(false)
            .open(path);

        self.file = Some(match file {
            Ok(file) => file,
            Err(error) => Err(LoggerError::GenericIOError(LOGGER_LOCATION.clone(), error))?, 
        });

        Ok(self)
    }

    pub fn log(&mut self, msg: impl Into<Arc<str>> + std::fmt::Display) -> Result<(),LoggerError> { 
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
            None => Err(LoggerError::Uninitialized)?
        };

        match write {
            Ok(_) => Ok(()),
            Err(error) => Err(LoggerError::GenericIOError(LOGGER_LOCATION.clone(), error)),
        }
    }
}
