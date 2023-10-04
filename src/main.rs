use utils::arguments::{self, Arguments};


mod config;
mod exec;
mod constants;
mod utils;
mod compat;
mod sync;
mod log;

#[derive(Clone, Copy)]
enum Options {
    Sync,
    Remove,
    Query,
    Compat,
    Interpose,
    Exec,
    BashProc,
    BashHelp,
    BashUtils,
    Version,
    None,
}

fn main() {
    let mut option: Options = Options::None;

    Arguments::new()
        .map(&mut option)
        .short("-Q").long("--query").set(Options::Query)
        .long("--fake-chroot").set(Options::Sync)
        .short("-S").long("--sync").set(Options::Interpose)
        .short("-R").long("--remove").set(Options::Remove)
        .short("-E").long("--exec").set(Options::Exec)
        .short("-V").long("--version").set(Options::Version)
        .short("-Axc").long("--aux-compat").set(Options::Compat)
        .short("-P").long("--proc").set(Options::BashProc)
        .short("-H").long("--help").set(Options::BashHelp)
        .short("-U").long("--utils").set(Options::BashUtils)
        .parse_arguments();

    match option {
        Options::Exec => exec::execute(),
        Options::Sync => sync::synchronize(),
        Options::Interpose => sync::interpose(), 
        Options::Query => sync::query(),
        Options::Remove => sync::remove(),
        Options::Compat => compat::compat(),
        Options::Version => arguments::print_version(),
        Options::BashUtils => compat::execute_bash("utils"),
        Options::BashProc => compat::execute_bash("ps"), 
        Options::BashHelp => compat::execute_bash("man"),
        Options::None => arguments::invalid(),
    }
}
