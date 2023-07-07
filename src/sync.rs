#![allow(dead_code)]

use std::collections::HashMap;
use alpm::{Alpm,  SigLevel, Usage, PackageReason};
use console::style;
use lazy_static::lazy_static;
use pacmanconf;

use crate::constants;
use crate::sync::dl_event::DownloadCallback;
use crate::sync::linker::Linker;
use crate::sync::progress_event::ProgressCallback;
use crate::sync::update::Update;
use crate::utils::{Arguments, test_root};
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
    let args = Arguments::new(1, "-NS", HashMap::from([("--sync".into(), "y".into()), 
                                                       ("--update".into(), "u".into()), 
                                                       ("--query".into(),"q".into()),]));
        
    let cache: InstanceCache = InstanceCache::new();
    let switch = args.get_switch();

    if switch.starts_with("q") { 
        query_database(&args) 
    } else {

        let mut u: Update = Update::new();

        if switch.contains("y") {
            synchronize_database(&cache);
        } 
        if switch.contains("u") {

            u.update(&cache, &cache.containers_base());
            u.update(&cache, &cache.containers_dep());
       
            if u.updated().len() > 0 {
                let mut l: Linker = Linker::new(cache.registered().len());
                linker::wait_on(l.link(&cache, &cache.registered(), Vec::new()));
                l.finish();
            }
            
            u.update(&cache, &cache.containers_root());
        }

        println!("{} Transaction complete.",style("->").bold().green());
    }
}

fn query_database(args: &Arguments) {    
    let instance = args.get_targets()[0].clone();
    let instance_vars = InsVars::new(&instance);

    test_root(&instance_vars);

    let root = instance_vars.root().as_str(); 
    let handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", instance_vars.root())).unwrap();

    for pkg in handle.localdb().pkgs() {
        if pkg.reason() != PackageReason::Explicit && args.get_switch().contains("e") {
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



fn synchronize_database(cache: &InstanceCache) {
    for i in cache.registered().iter() {
        let ins: &InstanceHandle = cache.instances().get(i).unwrap();
        if ins.instance().container_type() == "BASE" {
            let mut handle = instantiate_alpm_syncdb(&ins);
    
             println!("{} {} ",style("::").bold().green(), style("Synchronising package databases...").bold()); 
            handle.syncdbs_mut().update(false).unwrap();
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
