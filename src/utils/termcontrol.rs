use nix::sys::termios::{Termios, 
    tcgetattr, 
    tcsetattr, 
    SetArg::TCSANOW};

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
            Ok(output) => Self { 
                tm: Some(output), 
                fd: f
            },
            Err(_) => Self { 
                tm: None, 
                fd: f
            }
        }
    }

   /* 
    * Check if Termios initiated and then execute tcsetattr to reset terminal.
    */

    pub fn reset_terminal(&self) -> Result<(), ()>  {
        if let Some(tm) = &self.tm {
            match tcsetattr(self.fd, TCSANOW, &tm) {
                Ok(_) => Ok(()),
                Err(_) => Err(())
            }
        } else {
            Err(())
        }
    }
}

