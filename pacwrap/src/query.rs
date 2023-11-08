use std::process::exit;
use alpm::{Alpm, PackageReason};

use pacwrap_lib::{config,
    constants::{RESET, BOLD_GREEN},
    utils::arguments::Operand,
    utils::{arguments::Arguments, 
    print_error,
    print_help_error}};

pub fn query(arguments: &mut Arguments) {
    let mut target = "";
    let mut explicit = false;
    let mut quiet = false;

    while let Some(arg) = arguments.next() {
        match arg {
            Operand::Short('e') | Operand::Long("explicit") => explicit = true,
            Operand::Short('q') | Operand::Long("quiet") => quiet = true,
            Operand::LongPos("target", t) | Operand::ShortPos(_, t) => target = t,
            _ => print_help_error(arguments.invalid_operand()),
        }
    }

    if target.is_empty() {
        print_help_error("Target not specified.");
    }

    match config::provide_handle(target) {
        Ok(handle) => {
            let root = handle.vars().root().as_ref(); 
            let handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", root)).unwrap();

            for pkg in handle.localdb().pkgs() {
                if explicit && pkg.reason() != PackageReason::Explicit {
                    continue;
                }
        

                match quiet {
                    true => println!("{} ", pkg.name()),
                    false => println!("{} {}{}{} ", pkg.name(), *BOLD_GREEN, pkg.version(), *RESET), 
                } 
            }
        },
        Err(error) => {
            print_error(error);
            exit(1);
        }
    }
}
