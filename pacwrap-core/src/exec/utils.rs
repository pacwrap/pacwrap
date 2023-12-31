use std::process::Child;

use crate::{constants::BWRAP_EXECUTABLE, config::InstanceHandle, ErrorKind, error::*, err};

pub fn execute_in_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<()> {
    match super::fakeroot_container(ins, arguments) {
        Ok(mut child) => match child.wait() {
            Ok(_) => Ok(()),
            Err(err) => err!(ErrorKind::ProcessWaitFailure(BWRAP_EXECUTABLE, err.kind()))
        },
        Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())),
    }
}

pub fn handle_process(name: &str, result: std::result::Result<Child, std::io::Error>) -> Result<()> {
    match result {
        Ok(child) => Ok(wait_on_process(child)),
        Err(error) => err!(ErrorKind::IOError(name.into(), error.kind())),
    }
}

fn wait_on_process(mut child: Child) { 
    child.wait().ok(); 
}
