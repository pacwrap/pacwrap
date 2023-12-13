use std::{collections::HashMap, fs::File};
use std::io::Write;

use indexmap::IndexMap;
use pacwrap_core::{ErrorKind,
    log::Logger,
    sync::transaction::TransactionType,
    utils::arguments::{Arguments, 
        InvalidArgument, 
        Operand},
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

    if create(&mut args)  {
        if let TransactionType::Upgrade(upgrade, refresh, _) = action { 
            if ! refresh {
                Err(ErrorKind::Argument(InvalidArgument::UnsuppliedOperand("--refresh", "Required for container creation.")))?
            } else if ! upgrade {
                Err(ErrorKind::Argument(InvalidArgument::UnsuppliedOperand("--upgrade", "Required for container creation.")))?
            }
        }

        instantiate(acquire_depends(args)?)?
    }

    engage_aggregator(action, &mut args, &mut logger)
}

fn acquire_depends<'a>(args: &'a mut Arguments) -> Result<IndexMap<&'a str, (InstanceType, Vec<&'a str>)>, ErrorKind> {
    let mut deps: IndexMap<&'a str, (InstanceType, Vec<&'a str>)> = IndexMap::new();
    let mut current_target = "";
    let mut instype = None;

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::ShortPos('d', dep) 
            | Operand::LongPos("dep", dep) => match deps.get_mut(current_target) {
                Some(d) => {     
                    if let Some(instype) = instype {
                        if let InstanceType::BASE = instype {
                            Err(ErrorKind::Message("Dependencies cannot be assigned to base containers."))?
                        }
                    }
                         
                    d.1.push(dep); 
                },
                None => Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))?
            },
            Operand::Short('b') 
            | Operand::Long("base") => instype = Some(InstanceType::BASE),
            Operand::Short('s') 
            | Operand::Long("slice") => instype = Some(InstanceType::DEP),
            Operand::Short('r') 
            | Operand::Long("root") => instype = Some(InstanceType::ROOT),
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) => match instype {
                    Some(instype) => {
                        current_target = target;
                        deps.insert(current_target, (instype, vec!()));
                    },
                    None => Err(ErrorKind::Message("Container type not specified."))?,
            },          
            _ => continue,
        }
    }

    if current_target.len() == 0 {
        Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))?
    }

    Ok(deps)
}


fn create(args: &mut Arguments) -> bool {
    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('c') | Operand::Long("create") => return true, 
            _ => continue,
        } 
    }

    false
}

fn instantiate<'a>(targets: IndexMap<&'a str, (InstanceType, Vec<&'a str>)>) -> Result<(), ErrorKind> { 
    println!("{} {}Instantiating container{}{}", *BAR_GREEN, *BOLD, if targets.len() > 1 { "s" } else { "" }, *RESET);

    for target in targets {
        instantiate_container(target.0, target.1.0, target.1.1)?;
    }

    Ok(())
}

fn instantiate_container(ins: &str, instype: InstanceType, deps: Vec<&str>) -> Result<(), ErrorKind> {
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
            Err(ErrorKind::Message(format!("Container {ins} already exists.").leak()))?
        } else {
            Err(ErrorKind::IOError(instance.vars().root().into(), err.kind()))? 
        }    
    }

    if let InstanceType::ROOT | InstanceType::BASE = instype { 
        if let Err(err) = std::fs::create_dir(instance.vars().home()) {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                Err(ErrorKind::IOError(instance.vars().root().into(), err.kind()))?
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
    println!("{} Instantiation of {ins} complete.", *ARROW_GREEN);
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
                Operand::Short('d') 
                | Operand::Long("dep") | Operand::LongPos("dep", _)
                | Operand::Short('s') | Operand::Long("slice")
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

                if base {         
                    queue.insert(current_target.into(), vec!("base", "pacwrap-base-dist")); 
                    base = false;  
                }
            },
            Operand::LongPos(_, package) 
            | Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { queue.insert(current_target.into(), vec!(package)); },
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
