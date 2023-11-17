use pacwrap_core::{config, 
        utils::{arguments::{Arguments, Operand}, 
        print_help_error}};

mod sync;
mod remove;
mod query;
mod exec;
mod compat;
mod manual;

fn main() {
    let arguments = &mut Arguments::new().parse();
    let param = arguments.next().unwrap_or_default();

    match param {
        Operand::Short('S') | Operand::Long("sync") => (), _ => config::init::init(),
    }

    match param {
        Operand::Short('E') | Operand::Long("exec") => exec::execute(arguments),
        Operand::Short('S') | Operand::Long("sync") => sync::synchronize(arguments), 
        Operand::Short('R') | Operand::Long("remove") => remove::remove(arguments),
        Operand::Short('Q') | Operand::Long("query") => query::query(arguments),
        Operand::Short('U') | Operand::Long("utils") => compat::execute_bash("utils", arguments),
        Operand::Short('P') | Operand::Long("proc") => compat::execute_bash("ps", arguments), 
        Operand::Short('h') | Operand::Long("help") => manual::help(arguments),
        Operand::Short('V') | Operand::Long("version") => manual::print_version(arguments),
        Operand::Long("compat") => compat::compat(arguments),
        _ => print_help_error(arguments.invalid_operand()),
    }
}
