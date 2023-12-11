use std::collections::HashMap;

use pacwrap_core::{log::Logger,
    sync::transaction::TransactionType,
    utils::arguments::Operand,
    utils::arguments::{Arguments, InvalidArgument},
    config::cache,
    sync::transaction::{TransactionFlags, TransactionAggregator}, ErrorKind};

pub fn remove(mut args: &mut Arguments) -> Result<(), ErrorKind> {
    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let action = {
        let mut recursive = 0;
        let mut cascade = false;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('s') | Operand::Long("recursive") => recursive += 1,
                Operand::Short('c') | Operand::Long("cascade") => cascade = true,
                _ => continue,
            }
        }

        TransactionType::Remove(recursive > 0 , cascade, recursive > 1) 
    };
    
    engage_aggregator(action, &mut args, &mut logger)
}

fn engage_aggregator<'a>(
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    log: &'a mut Logger) -> Result<(), ErrorKind> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str,Vec<&'a str>> = HashMap::new();
    let mut current_target = None;

    if let Operand::None = args.next().unwrap_or_default() { 
        Err(ErrorKind::Argument(InvalidArgument::OperationUnspecified))?
    }

    while let Some(arg) = args.next() {
        match arg {
            Operand::Long("remove")
                | Operand::Long("cascade") 
                | Operand::Long("recursive") 
                | Operand::Short('R')
                | Operand::Short('c')  
                | Operand::Short('s') 
                | Operand::Short('t') => continue,  
            Operand::Long("noconfirm") 
                => action_flags = action_flags | TransactionFlags::NO_CONFIRM,                  
            Operand::Short('p') 
                | Operand::Long("preview") 
                => action_flags = action_flags | TransactionFlags::PREVIEW, 
            Operand::Long("db-only") 
                => action_flags = action_flags | TransactionFlags::DATABASE_ONLY,
            Operand::Long("force-foreign") 
                => action_flags = action_flags | TransactionFlags::FORCE_DATABASE,
            Operand::Short('f') 
                | Operand::Long("filesystem") 
                => action_flags = action_flags | TransactionFlags::FILESYSTEM_SYNC, 
            Operand::ShortPos('t', target) 
                | Operand::LongPos("target", target) 
                | Operand::ShortPos(_, target) => {
                current_target = Some(target);
                targets.push(target);
            },
            Operand::Value(package) => if let Some(target) = current_target {
                match queue.get_mut(target) {
                    Some(vec) => vec.push(package),
                    None => { queue.insert(target, vec!(package)); },
                }
            },
            _ => Err(args.invalid_operand())?,
        }
    }
        
    if let None = current_target {
        Err(ErrorKind::Argument(InvalidArgument::TargetUnspecified))?
    }

    Ok(TransactionAggregator::new(&cache::populate()?, 
        queue, 
        log, 
        action_flags, 
        action_type, 
        current_target)
        .aggregate()?)
}
