use alpm::{Alpm, PackageReason};

use pacwrap_core::{config,
    constants::{RESET, BOLD_GREEN},
    utils::arguments::{Operand, InvalidArgument},
    utils::arguments::Arguments,
    error::*, 
    err};

pub fn query(arguments: &mut Arguments) -> Result<()> {
    let mut target = "";
    let mut explicit = false;
    let mut quiet = false;

    while let Some(arg) = arguments.next() {
        match arg {
            Operand::Short('e') | Operand::Long("explicit") => explicit = true,
            Operand::Short('q') | Operand::Long("quiet") => quiet = true,
            Operand::LongPos("target", t) | Operand::ShortPos(_, t) => target = t,
            _ => arguments.invalid_operand()?,
        }
    }

    if target.is_empty() {
        err!(InvalidArgument::TargetUnspecified)?
    }

    let handle = config::provide_handle(target)?;
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
    
    Ok(())
}
