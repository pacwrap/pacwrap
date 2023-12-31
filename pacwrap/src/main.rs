use pacwrap_core::utils::arguments::{Arguments, Operand};

mod sync;
mod remove;
mod query;
mod exec;
mod compat;
mod manual;

fn main() {
    let arguments = &mut Arguments::new().populate();
    let result = match arguments.next().unwrap_or_default() {
        Operand::Short('E') | Operand::Long("exec") => exec::execute(arguments),
        Operand::Short('S') | Operand::Long("sync") => sync::synchronize(arguments), 
        Operand::Short('R') | Operand::Long("remove") => remove::remove(arguments),
        Operand::Short('Q') | Operand::Long("query") => query::query(arguments),
        Operand::Short('U') | Operand::Long("utils") => compat::execute_bash("pacwrap-utils", arguments),
        Operand::Short('P') | Operand::Long("proc") => compat::execute_bash("pacwrap-ps", arguments), 
        Operand::Short('h') | Operand::Long("help") => manual::help(arguments),
        Operand::Short('V') | Operand::Long("version") => manual::print_version(arguments),
        Operand::Long("compat") => compat::compat(arguments),
        _ => arguments.invalid_operand(),
    };

    if let Err(error) = result {
        error.handle();
    }
}
