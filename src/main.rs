use utils::{arguments::{Arguments, Operand}, print_help_error};

mod config;
mod exec;
mod constants;
mod utils;
mod compat;
mod sync;
mod log;
mod manual;

fn main() {
    let arguments = &mut Arguments::new().parse();
    let param = arguments.next().unwrap_or_default();

    match param {
        Operand::Short('S') | Operand::Long("sync") => (), _ => config::init::init(),
    }

    match param {
        Operand::Short('E') | Operand::Long("exec") => exec::execute(arguments),
        Operand::Short('Q') | Operand::Long("query") => sync::query(arguments),
        Operand::Short('R') | Operand::Long("remove") => sync::remove(arguments),
        Operand::Long("fake-chroot") => sync::synchronize(arguments),
        Operand::Short('S') | Operand::Long("sync") => sync::interpose(),  
        Operand::Short('U') | Operand::Long("utils") => compat::execute_bash("utils", arguments),
        Operand::Short('P') | Operand::Long("proc") => compat::execute_bash("ps", arguments), 
        Operand::Short('h') | Operand::Long("help") => manual::help(arguments),
        Operand::Short('V') | Operand::Long("version") => manual::print_version(arguments),
        Operand::Long("compat") => compat::compat(arguments), 
        _ => print_help_error(arguments.invalid_operand()),
    }
}
