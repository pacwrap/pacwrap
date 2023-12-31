use pacwrap_core::{err, Error, utils::{Arguments, arguments::Operand}};

use crate::error::AgentError;

mod error;
mod agent;

fn main() {
    let arguments = &mut Arguments::new().populate();
    let param = arguments.next().unwrap_or_default();
    let result = match param {
        Operand::Value("transact") => agent::transact(), _ => err!(AgentError::DirectExecution) 
    };
 
    if let Err(error) = result {
        error.handle();
    }
}
