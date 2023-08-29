use nix::{sys::termios::{Termios, 
    tcgetattr, 
    tcsetattr, 
    SetArg::TCSANOW}, 
    errno::Errno};

/*******
    *
    * ermControl struct
    * 
    * Impelments basic, portable functionalily for controlling terminal parameters.
    *
   ***/

pub struct TermControl {
    tm: Option<Termios>,
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

    pub fn new(f: i32) -> Self {
        match tcgetattr(f) {
            Ok(t) => Self { tm: Some(t), fd: f},
            Err(_) => Self { tm: None, fd: f}
        }
    }

   /* 
    * Check if Termios initiated and then execute tcsetattr to reset terminal.
    */

    pub fn reset_terminal(&self) -> Result<(), Errno>  {
        match self.tm.as_ref() {
            Some(tm) => tcsetattr(self.fd, TCSANOW, tm),
            None => Ok(())
        }
    }
}

