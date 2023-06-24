use std::mem::zeroed;

use nix::sys::termios::{Termios, tcgetattr, tcsetattr, SetArg::TCSANOW};

/*******
    *
    * ermControl struct
    * 
    * Impelments basic, portable functionalily for controlling terminal parameters.
    *
   ***/

pub struct TermControl {
    tm: Termios,
    init: bool,
    fd: i32 
}

impl TermControl {
    /*
     * A valid termios struct is presumed to be returned 
     * if there is a valid tty at specified fd. 
     *
     * If the application is not being instantiated from a tty, 
     * then return a zeroed struct.
     */

    pub fn new(fd: i32) -> Self {
        match tcgetattr(fd) {
            Ok(output) => Self { tm: output, init: true, fd: fd},
            Err(_) => Self { tm: unsafe { zeroed() }, init: false, fd: fd}
        }
    }

   /* 
    * Check if Termios initiated and then execute tcsetattr to reset terminal.
    */

    pub fn reset_terminal(&self) -> Result<(), ()>  {
        if self.init {
            match tcsetattr(self.fd, TCSANOW, &self.tm) {
                Ok(_) => Ok(()),
                Err(_) => Err(())
            }
        } else {
            Err(())
        }
    }
}

