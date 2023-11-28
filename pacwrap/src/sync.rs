use std::{collections::HashMap, fs::File};
use std::io::Write;
use std::process::exit;

use pacwrap_core::{ErrorKind,
    log::Logger,
    sync::transaction::TransactionType,
    utils::{arguments::{Arguments, 
        InvalidArgument, 
        Operand}, 
        print_error},
    config::{self, 
        InstanceType, 
        InsVars, 
        Instance, 
        InstanceHandle,
        cache},
    sync::transaction::{TransactionFlags, TransactionAggregator}, 
    constants::{BAR_GREEN, BOLD, RESET, ARROW_GREEN}};

pub fn synchronize(mut args: &mut Arguments) -> Result<(), ErrorKind> {
    let mut logger = Logger::new("pacwrap-sync").init().unwrap();  
    let action = {
        let mut u = 0;
        let mut y = 0;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('y') | Operand::Long("refresh") => y += 1,
                Operand::Short('u') | Operand::Long("upgrade") => u += 1,
                _ => continue,
            }
        }

        TransactionType::Upgrade(u > 0, y > 0, y > 1)
    };

    config::init::init();

    if let Some(instype) = create(&mut args)?  {
        if let TransactionType::Upgrade(upgrade, refresh, _) = action { 
            if ! refresh {
                Err(ErrorKind::Argument(InvalidArgument::UnsuppliedOperand("--refresh", "Required for container creation.")))?
            } else if ! upgrade {
                Err(ErrorKind::Argument(InvalidArgument::UnsuppliedOperand("--upgrade", "Required for container creation.")))?
            }
        }

        instantiate(instype, args.targets())?
    }

    engage_aggregator(action, &mut args, &mut logger)
}

fn create<'a>(args: &mut Arguments) -> Result<Option<InstanceType>, ErrorKind> {
    let mut instype = None;
    let mut create = false;

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('c') | Operand::Long("create") => create = true, 
            Operand::Short('b') | Operand::Long("base") => match instype { 
                None => instype =  Some(InstanceType::BASE),
                Some(_) => Err(ErrorKind::DuplicateType)?,
            },
            Operand::Short('d') | Operand::Long("slice") => match instype {
                None => instype = Some(InstanceType::DEP),
                Some(_) => Err(ErrorKind::DuplicateType)?, 
            },
            Operand::Short('r') | Operand::Long("root") => match instype {
                None => instype = Some(InstanceType::ROOT), 
                Some(_) => Err(ErrorKind::DuplicateType)?,
            },
            _ => continue,
        } 
    }

    match create { 
        true => match instype {
            None => Err(ErrorKind::Message("Container type unspecified."))?, Some(_) => Ok(instype),
        },
        false => Ok(None) 
    }
}

fn instantiate(instype: InstanceType, mut targets: Vec<&str>) -> Result<(), ErrorKind> {
    if targets.len() == 0 {
        Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))?
    }

    let target = targets.remove(0);

    if let InstanceType::ROOT | InstanceType::DEP = instype {
        if target.len() == 0 {
            Err(ErrorKind::Message("Dependencies not specified."))?         
        }
    }

    instantiate_container(target, targets, instype)
}

fn instantiate_container(ins: &str, deps: Vec<&str>, instype: InstanceType) -> Result<(), ErrorKind> {
    println!("{} {}Instantiating container {ins}{}", *BAR_GREEN, *BOLD, *RESET);

    let deps = deps.iter().map(|a| { let a = *a; a.into() }).collect();
    let mut logger = Logger::new("pacwrap").init().unwrap();
    let instance = match config::provide_new_handle(ins) {
        Ok(mut handle) => {
            handle.metadata_mut().set(deps, vec!());
            handle
        },
        Err(_) => {
            let vars = InsVars::new(ins);
            let cfg = Instance::new(instype, vec!(), deps);
            InstanceHandle::new(cfg, vars) 
        }
    };

    if let Err(err) = std::fs::create_dir(instance.vars().root()) {
        if let std::io::ErrorKind::AlreadyExists = err.kind() {
            Err(ErrorKind::Message("Container root already exists."))?
        } else {
            Err(ErrorKind::IOError(instance.vars().root().into(), err.kind()))? 
        }    
    }

    if let InstanceType::ROOT | InstanceType::BASE = instype { 
        if let Err(err) = std::fs::create_dir(instance.vars().home()) {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                print_error(format!("'{}': {}", instance.vars().root(), err));
                exit(1);
            }
        }

        let bashrc = format!("{}/.bashrc", instance.vars().home());
        
        match File::create(&bashrc) {
            Ok(mut f) => if let Err(error) = write!(f, "PS1=\"{}> \"", ins) {
                Err(ErrorKind::IOError(bashrc, error.kind()))?
            },
            Err(error) => Err(ErrorKind::IOError(bashrc.clone(), error.kind()))?
        }; 
    }

    config::save_handle(&instance).ok(); 
    logger.log(format!("Configuration file created for {ins}")).unwrap();
    drop(instance);
    println!("{} Instantiation complete.", *ARROW_GREEN);
    Ok(())
}

fn engage_aggregator<'a>(
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    log: &'a mut Logger) -> Result<(), ErrorKind> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str ,Vec<&'a str>> = HashMap::new();
    let mut current_target = "";
    let mut base = false;

    if let Operand::None = args.next().unwrap_or_default() {
        Err(ErrorKind::Argument(InvalidArgument::OperationUnspecified))?
    }

    while let Some(arg) = args.next() {
        match arg {
                Operand::Short('d') | Operand::Long("slice")
                | Operand::Short('r') | Operand::Long("root") 
                | Operand::Short('t') | Operand::Long("target") 
                | Operand::Short('y') | Operand::Long("refresh")
                | Operand::Short('u') | Operand::Long("upgrade") => continue,
            Operand::Short('o') 
                | Operand::Long("target-only") 
                => action_flags = action_flags | TransactionFlags::TARGET_ONLY,
            Operand::Short('f') 
                | Operand::Long("filesystem") 
                => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::Short('p') 
                | Operand::Long("preview") 
                => action_flags = action_flags | TransactionFlags::PREVIEW,
            Operand::Short('c') 
                | Operand::Long("create") 
                => action_flags = action_flags | TransactionFlags::CREATE 
                    | TransactionFlags::FORCE_DATABASE,
            Operand::Short('b') | 
                Operand::Long("base") => base = true,
            Operand::Long("db-only") 
                => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") 
                => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Long("noconfirm") 
                => action_flags = action_flags | TransactionFlags::NO_CONFIRM, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) => {
                current_target = target;
                targets.push(target);
            },
            Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { 
                        let packages = if base {
                            base = false;
                            vec!(package, "base", "pacwrap-base-dist")
                        } else {
                            vec!(package)
                        };

                        queue.insert(current_target.into(), packages); 
                    },
                }
            },
            _ => Err(args.invalid_operand())?
        }
    }

    let current_target = match action_flags.intersects(TransactionFlags::TARGET_ONLY) {
        true => {
            if current_target == "" && ! action_flags.intersects(TransactionFlags::FILESYSTEM_SYNC) {
                Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))?
            }

            Some(current_target)
        },
        false => {
            if let TransactionType::Upgrade(upgrade, refresh, _) = action_type {
                if ! upgrade && ! refresh {
                    Err(ErrorKind::Argument(InvalidArgument::OperationUnspecified))?
                }
            }
       
            None
        }
    };

    Ok(TransactionAggregator::new(&cache::populate()?, 
        queue, 
        log, 
        action_flags, 
        action_type, 
        current_target).aggregate()?)
}
