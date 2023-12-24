use pacwrap_core::utils::{print_error, Arguments, arguments::Operand};

mod agent;

fn main() {
    let arguments = &mut Arguments::new().populate();
    let param = arguments.next().unwrap_or_default();

    match param {
        Operand::Value("transact") => agent::transact(),
        _ => print_error("Direct execution of this binary is unsupported.") 
    }
}
