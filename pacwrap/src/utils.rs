use std::process::Command;

use pacwrap_core::{
    config,
    err,
    utils::{
        arguments::{InvalidArgument, Operand},
        Arguments,
    },
    Error,
    ErrorKind,
    Result,
};

pub mod delete;
pub mod desktop;
mod edit;
pub mod list;

const GIO: &'static str = "gio";

pub fn engage_utility(args: &mut Arguments) -> Result<()> {
    match args.next().unwrap_or_default() {
        Operand::Short('v') | Operand::Long("view") | Operand::Value("view") => edit::edit_file(args, false),
        Operand::Short('e') | Operand::Long("edit") | Operand::Value("edit") => edit::edit_file(args, true),
        Operand::Short('r') | Operand::Long("remove") | Operand::Value("remove") => delete::remove_containers(args),
        Operand::Short('d') | Operand::Long("desktop") | Operand::Value("desktop") => desktop::file(args),
        Operand::Short('l') | Operand::Long("list") | Operand::Value("ls") => list::list_containers(args),
        Operand::Short('o') | Operand::Long("open") | Operand::Value("open") => open(args),
        _ => args.invalid_operand(),
    }
}

fn open(args: &mut Arguments) -> Result<()> {
    enum DirectoryType {
        Home,
        Root,
    }

    let directory = match args.next().unwrap_or_default() {
        Operand::Short('h') | Operand::Long("home") | Operand::Value("home") => DirectoryType::Home,
        Operand::Short('r') | Operand::Long("root") | Operand::Value("root") => DirectoryType::Root,
        _ => return args.invalid_operand(),
    };
    let instance = config::provide_handle(match args.next().unwrap_or_default() {
        Operand::ShortPos('h', val) | Operand::LongPos("home", val) => val,
        Operand::ShortPos('r', val) | Operand::LongPos("root", val) => val,
        Operand::ShortPos('t', val) | Operand::LongPos("target", val) => val,
        Operand::Value(val) => val,
        _ => return err!(InvalidArgument::TargetUnspecified),
    })?;
    let directory = match directory {
        DirectoryType::Root => instance.vars().root(),
        DirectoryType::Home => instance.vars().home(),
    };

    match Command::new(GIO).arg("open").arg(directory).spawn() {
        Ok(_) => Ok(()),
        Err(err) => err!(ErrorKind::ProcessInitFailure(GIO, err.kind())),
    }
}
