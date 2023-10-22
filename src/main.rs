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
    let mut arguments = Arguments::new().parse();
    let param = arguments.next().unwrap_or_default();

    match param {
        Operand::Short('S') | Operand::Long("sync") => (), _ => config::init::init(),
    }

    match param {
        Operand::Short('E') | Operand::Long("exec") => exec::execute(&mut arguments),
        Operand::Short('Q') | Operand::Long("query") => sync::query(arguments),
        Operand::Short('R') | Operand::Long("remove") => sync::remove(arguments),
        Operand::Short('A') | Operand::Long("aux-compat") => compat::compat(arguments),
        Operand::Long("fake-chroot") => sync::synchronize(arguments),
        Operand::Short('S') | Operand::Long("sync") => sync::interpose(),  
        Operand::Short('U') | Operand::Long("utils") => compat::execute_bash("utils"),
        Operand::Short('P') | Operand::Long("proc") => compat::execute_bash("ps"), 
        Operand::Short('h') | Operand::Long("help") => manual::help(arguments),
        Operand::Short('V') | Operand::Long("version") => manual::print_version(arguments),
        Operand::None => print_help_error("Operation not specified."),
        _ => arguments.invalid_operand(),
    }
}
