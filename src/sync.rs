#![allow(dead_code)]

use alpm::{Alpm,  SigLevel, Usage, PackageReason};
use console::style;
use lazy_static::lazy_static;
use pacmanconf;

use crate::constants;
use crate::sync::dl_event::DownloadCallback;
use crate::sync::linker::Linker;
use crate::sync::progress_event::ProgressCallback;
use crate::sync::update::Update;
use crate::utils::{Arguments, test_root, print_help_msg};
use crate::config::InsVars;
use crate::config::cache::InstanceCache;
use crate::config::InstanceHandle;

lazy_static! {
    static ref PACMAN_CONF: pacmanconf::Config = pacmanconf::Config::from_file(format!("{}/pacman/pacman.conf", constants::LOCATION.get_config())).unwrap(); 
}

mod progress_event;
mod dl_event;
mod query_event;
mod linker;
mod update;

pub fn execute() { 
    let mut sync = false;
    let mut update = false;
    let mut query = false;
    let mut explicit = false;
    let mut sync_count = 0;
    let mut args = Arguments::new().prefix("-S")
        .switch("-y", "--sync", &mut sync).count(&mut sync_count)
        .switch("-u", "--upgrade", &mut update)
        .switch("-q", "--query", &mut query)
        .switch("-e", "--explicit", &mut explicit);
    
    args = args.parse_arguments();
    let targets = args.get_runtime().clone();
    let mut cache: InstanceCache = InstanceCache::new();

    if targets.len() > 0 {
        cache.populate_from(&targets);
    } else {
        cache.populate();
    }
 
    if sync && sync_count == 4 {      
        let mut l: Linker = Linker::new(); 
        l.start(cache.registered().len());
        linker::wait_on(l.link(&cache, cache.registered(), Vec::new()));
        l.finish();
    } else if query {
        if targets.len() < 1 {
            print_help_msg("Target not specified.");
        }
        query_database(targets.get(0).unwrap(), explicit) 
    } else {
        if sync {
            synchronize_database(&cache, sync_count > 1);
        }

        if update {
            update::update(Update::new(), &cache);
        }
    }
}

fn query_database(instance: &String, explicit: bool) {    
    let instance_vars = InsVars::new(instance);

    test_root(&instance_vars);

    let root = instance_vars.root().as_str(); 
    let handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", instance_vars.root())).unwrap();

    for pkg in handle.localdb().pkgs() {
        if pkg.reason() != PackageReason::Explicit && explicit {
            continue;
        }
        println!("{} ", pkg.name());
    } 
}

fn instantiate_alpm(inshandle: &InstanceHandle) -> Alpm { 
    let root = inshandle.vars().root().as_str();  
    test_root(&inshandle.vars());
    let mut handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", root)).unwrap();
    handle.set_cachedirs(vec![format!("{}/pkg",constants::LOCATION.get_cache())].iter()).unwrap();
    handle.set_parallel_downloads(parallel_downloads());   
    handle = register_remote(handle); handle
}

fn instantiate_alpm_syncdb(inshandle: &InstanceHandle) -> Alpm { 
    let root = inshandle.vars().root().as_str();  
    test_root(&inshandle.vars()); 
    let mut handle = Alpm::new2(root, &format!("{}/pacman/", constants::LOCATION.get_data())).unwrap();
    handle.set_cachedirs(vec![format!("{}/pkg",constants::LOCATION.get_cache())].iter()).unwrap(); 
    handle.set_progress_cb(ProgressCallback::new(false), progress_event::progress_event);
    handle.set_dl_cb(DownloadCallback::new(false), dl_event::download_event);   
    handle.set_parallel_downloads(parallel_downloads());    
    handle = register_remote(handle); handle
}

fn register_remote(mut handle: Alpm) -> Alpm { 
    for repo in PACMAN_CONF.repos.iter() {
        let core = handle
        .register_syncdb_mut(repo.name.as_str(), SigLevel::USE_DEFAULT)
        .unwrap();

        for server in repo.servers.iter() {
            core.add_server(server.as_str()).unwrap();
        }

        core.set_usage(Usage::SYNC | Usage::SEARCH).unwrap();
    }

    handle
}

fn synchronize_database(cache: &InstanceCache, force: bool) {
    for i in cache.registered().iter() {
        let ins: &InstanceHandle = cache.instances().get(i).unwrap();
        if ins.instance().container_type() == "BASE" {
            let mut handle = instantiate_alpm_syncdb(&ins);
    
            println!("{} {} ",style("::").bold().green(), style("Synchronising package databases...").bold()); 
            handle.syncdbs_mut().update(force).unwrap();
            Alpm::release(handle).unwrap();  
            break;
        }
    }

    for i in cache.registered().iter() {
        let ins: &InstanceHandle = cache.instances().get(i).unwrap();
        let vars: &InsVars = ins.vars();
        
        for repo in PACMAN_CONF.repos.iter() {
            let src = &format!("{}/pacman/sync/{}.db",constants::LOCATION.get_data(), repo.name);
            let dest = &format!("{}/var/lib/pacman/sync/{}.db", vars.root(), repo.name);
            linker::create_hard_link(src, dest);
        }
    } 
}

fn parallel_downloads() -> u32 {
    match PACMAN_CONF.parallel_downloads.try_into() { Ok(i) => i, Err(_) => 1 }
}
