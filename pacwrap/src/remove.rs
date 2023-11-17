use std::collections::HashMap;

use pacwrap_core::{log::Logger,
    sync::transaction::TransactionType,
    utils::{arguments::Operand, print_help_error},
    utils::arguments::Arguments,
    config::cache::InstanceCache,
    sync::transaction::{TransactionFlags, TransactionAggregator}};

pub fn remove(mut args: &mut Arguments) {
    let mut cache: InstanceCache = InstanceCache::new();

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
    
    match ascertain_aggregator(action, &mut args, &mut cache, &mut logger) {
        Ok(ag) => ag.aggregate(&mut InstanceCache::new()), Err(e) => print_help_error(e),
    }
}

fn ascertain_aggregator<'a>(
    action_type: TransactionType, 
    args: &'a mut Arguments, 
    inscache: &'a mut InstanceCache, 
    log: &'a mut Logger) -> Result<TransactionAggregator<'a>, String> { 
    let mut action_flags = TransactionFlags::NONE;
    let mut targets = Vec::new();
    let mut queue: HashMap<&'a str,Vec<&'a str>> = HashMap::new();
    let mut current_target = "";

    args.set_index(1);

    if let Operand::None = args.next().unwrap_or_default() {
        Err("Operation not specified.")?
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
                current_target = target;
                targets.push(target.into());
            },
            Operand::Value(package) => if current_target != "" {
                match queue.get_mut(current_target.into()) {
                    Some(vec) => vec.push(package.into()),
                    None => { queue.insert(current_target, vec!(package)); },
                }
            },
            _ => Err(args.invalid_operand())?,
        }
    }
        
    if current_target == "" {
        Err("Target not specified")?
    }

    let current_target = Some(current_target);

    if targets.len() > 0 {
        inscache.populate_from(&targets, true);
    } else {
        inscache.populate();
    }

    Ok(TransactionAggregator::new(inscache, queue, log, action_flags, action_type, current_target))
}
