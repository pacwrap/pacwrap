use std::{collections::HashMap, fs::File};
use std::io::Write;
use std::process::exit;

use pacwrap_core::{log::Logger,
    sync::transaction::TransactionType,
    utils::{arguments::Operand, print_help_error, print_error},
    utils::arguments::Arguments,
    config::{self, 
        InstanceType, 
        InsVars, 
        Instance, 
        InstanceHandle,
        cache::InstanceCache},
    sync::transaction::{TransactionFlags, TransactionAggregator}, 
    constants::{BAR_GREEN, BOLD, RESET, ARROW_GREEN}};

pub fn synchronize(mut args: &mut Arguments) {
    let mut cache = InstanceCache::new();
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

    match create(&mut args) {
        Ok(option) => if let Some(instype) = option {
            if let TransactionType::Upgrade(upgrade, refresh, _) = action { 
                if ! upgrade {
                    print_help_error("--upgrade/-u not supplied with --create/-c.");
                } else if ! refresh {
                    print_help_error("--refresh/-y not supplied with --create/-c.");
                }
            }

            instantiate(instype, args.targets());
        },
        Err(error) => print_help_error(error),
    }

    match ascertain_aggregator(action, &mut args, &mut cache, &mut logger) {
        Ok(ag) => ag.aggregate(&mut InstanceCache::new()), Err(e) => print_help_error(e)
    }
}

fn create<'a>(args: &mut Arguments) -> Result<Option<InstanceType>, &'a str> {
    let mut instype = None;
    let mut create = false;

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('c') | Operand::Long("create") => create = true, 
            Operand::Short('b') | Operand::Long("base") => match instype { 
                None => instype =  Some(InstanceType::BASE),
                Some(_) => Err("Multiple container types cannot be assigned to a container.")?,
            },
            Operand::Short('d') | Operand::Long("slice") => match instype {
                None => instype = Some(InstanceType::DEP),
                Some(_) => Err("Multiple container types cannot be assigned to a container.")?,
            },
            Operand::Short('r') | Operand::Long("root") => match instype {
                None => instype = Some(InstanceType::ROOT),
                Some(_) => Err("Multiple container types cannot be assigned to a container.")?,
            },
            _ => continue,
        } 
    }

    match create { 
        true => match instype {
            None => Err("Instance type not specified"), Some(_) => Ok(instype),
        },
        false => Ok(None) 
    }
}

fn instantiate(instype: InstanceType, mut targets: Vec<&str>) {
    if targets.len() == 0 {
        print_help_error("Creation target not specified.");
    }

    let target = targets.remove(0);

    if let InstanceType::ROOT | InstanceType::DEP = instype {
        if target.len() == 0 {
            print_help_error("Dependency targets not specified.");
        }
    }

    instantiate_container(target, targets, instype); 
}

fn instantiate_container(ins: &str, deps: Vec<&str>, instype: InstanceType) {
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

    if let Err(err) = std::fs::create_dir(instance.vars().root().as_ref()) {
        if let std::io::ErrorKind::AlreadyExists = err.kind() {
            print_error(format!("'{}': Container root already exists.", instance.vars().root().as_ref()));
        } else {
            print_error(format!("'{}': {}", instance.vars().root().as_ref(), err));
        }
        
        exit(1);
    }

    if let InstanceType::ROOT | InstanceType::BASE = instype { 
        if let Err(err) = std::fs::create_dir(instance.vars().home().as_ref()) {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                print_error(format!("'{}': {}", instance.vars().root().as_ref(), err));
                exit(1);
            }
        }

        let mut f = match File::create(&format!("{}/.bashrc", instance.vars().home().as_ref())) {
            Ok(f) => f,
            Err(error) => {
                print_error(format!("'{}/.bashrc': {}", instance.vars().home().as_ref(), error));
                exit(1); 
            }
        };
   
        if let Err(error) = write!(f, "PS1=\"{}> \"", ins) {
            print_error(format!("'{}/.bashrc': {}", instance.vars().home().as_ref(), error));
            exit(1);
        }
    }

    config::save_handle(&instance).ok(); 
    logger.log(format!("Configuration file created for {ins}")).unwrap();
    drop(instance);
    println!("{} Instantiation complete.", *ARROW_GREEN);
}

fn ascertain_aggregator<'a>(
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    inscache: &'a mut InstanceCache, 
    log: &'a mut Logger) -> Result<TransactionAggregator<'a>, String> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str ,Vec<&'a str>> = HashMap::new();
    let mut current_target = "";
    let mut base = false;

    if let Operand::None = args.next().unwrap_or_default() {
        Err("Operation not specified.")?
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
                targets.push(target.into());
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
                Err("Target not specified")?
            }

            Some(current_target)
        },
        false => {
            if let TransactionType::Upgrade(upgrade, refresh, _) = action_type {
                if ! upgrade && ! refresh {
                    Err("Operation not specified.")? 
                }
            }
       
            None
        }
    };
 
    if targets.len() > 0 {
        inscache.populate_from(&targets, true);
    } else {
        inscache.populate();
    }
    
    Ok(TransactionAggregator::new(inscache, queue, log, action_flags, action_type, current_target))
}
