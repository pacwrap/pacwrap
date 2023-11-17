use pacwrap_core::utils::{print_error, Arguments, arguments::Operand};
use serde::{Serialize, Deserialize};

mod agent;

fn main() {
    let arguments = &mut Arguments::new().parse();
    let param = arguments.next().unwrap_or_default();

    match param {
        Operand::Value("transact") => agent::transact(),
        _ => print_error(arguments.invalid_operand())
    }
}


#[derive(Serialize, Deserialize, Clone)]
struct Test {
    string: String,
    version_major: i16,
    version_minor: i16,
}
