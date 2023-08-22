use std::process::exit;

use alpm::{Alpm,  SigLevel, Usage, PackageReason};
use console::style;
use lazy_static::lazy_static;
use pacmanconf;

use crate::constants::{self, LOCATION};
use crate::sync::{
    dl_event::DownloadCallback,
    linker::Linker,
    progress_event::ProgressCallback,
    transaction::TransactionType,
    transaction::TransactionAggregator};
use crate::utils::{Arguments, 
    arguments::invalid, 
    test_root,
    print_warning,
    print_error,
    print_help_msg};
use crate::config::{InsVars,
    InstanceHandle,
    cache::InstanceCache};

lazy_static! {
    static ref PACMAN_CONF: pacmanconf::Config = pacmanconf::Config::from_file(format!("{}/pacman/pacman.conf", constants::LOCATION.get_config())).unwrap(); 
    static ref DEFAULT_SIGLEVEL: SigLevel = signature(&PACMAN_CONF.sig_level, SigLevel::PACKAGE | SigLevel::DATABASE_OPTIONAL);
}

mod progress_event;
mod dl_event;
mod query_event;
mod linker;
mod transaction;
mod resolver;
mod resolver_local;
mod utils;

pub fn execute() { 
    validate_environment();

    let mut force_database = false;
    let mut search = false;
    let mut refresh = false;
    let mut upgrade = false;
    let mut preview = false;
    let mut no_confirm = false;
    let mut no_deps = false;
    let mut dbonly = false;
    let mut y_count = 0;

    let mut args = Arguments::new().prefix("-S")
        .ignore("--sync").ignore("--fake-chroot")
        .switch_big("--force-foreign", &mut force_database) 
        .switch_big("--db-only", &mut dbonly)
        .switch_big("--noconfirm", &mut no_confirm) 
        .switch("-y", "--refresh", &mut refresh).count(&mut y_count)
        .switch("-u", "--upgrade", &mut upgrade)
        .switch("-s", "--search", &mut search)
        .switch("-p", "--preview", &mut preview)
        .switch("-o", "--target-only", &mut no_deps);

    args = args.parse_arguments();
    let mut targets = args.targets().clone();
    let runtime = args.get_runtime().clone();
    let mut cache: InstanceCache = InstanceCache::new();
    let mut aux_cache: InstanceCache = InstanceCache::new();

    if targets.len() > 0 {
        cache.populate_from(&targets, true);
    } else {
        cache.populate();
    }
 
    if refresh && y_count == 4 {      
        let mut l: Linker = Linker::new(&cache); 
        l.start(cache.registered().len());
        l.link(&cache.registered(), 0);
        l.finish();
    } else if search {
        print_help_msg("Functionality is currently unimplemented.");
    } else {
        if refresh { 
            synchronize_database(&cache, y_count == 2); 
        }

        if upgrade || targets.len() > 0 {
            let mut update: TransactionAggregator = TransactionAggregator::new(TransactionType::Upgrade(upgrade), &cache)
                .preview(preview)
                .force_database(force_database)
                .database_only(y_count > 2 || dbonly)
                .no_confirm(no_confirm);
           
            if targets.len() > 0 { 
                let target = targets.remove(0);
                let inshandle = cache.instances().get(&target).unwrap();
                update.queue(target, runtime);
                if no_deps || ! upgrade {
                    update.transact(inshandle);
                } else {
                    transaction::update(update, &cache, &mut aux_cache); 
                }
            } else {
                transaction::update(update, &cache, &mut aux_cache);
            }
        } else if ! refresh {
            invalid();
        }
    }
}

pub fn remove() { 
    let mut preview = false;
    let mut recursive = false;
    let mut cascade = false;
    let mut no_confirm = false;
    let mut db_only = false;

    let mut args = Arguments::new().prefix("-R").ignore("--remove")
        .switch("-p", "--preview", &mut preview)
        .switch("-s", "--recursive", &mut recursive)
        .switch("-c", "--cascade", &mut cascade)
        .switch_big("--db-only", &mut db_only)
        .switch_big("--noconfirm", &mut no_confirm);
      
    args = args.parse_arguments();
    let mut targets = args.targets().clone();
    let runtime = args.get_runtime().clone();
    let mut cache: InstanceCache = InstanceCache::new();

    if targets.len() > 0 {
        cache.populate_from(&targets, true);
    } else {
        invalid();
    }

    let target = targets.remove(0);
    let inshandle = cache.instances().get(&target).unwrap();
    let mut update: TransactionAggregator = TransactionAggregator::new(TransactionType::Remove(recursive, cascade), &cache)
        .preview(preview)
        .database_only(db_only)
        .no_confirm(no_confirm);

    update.queue(target, runtime);
    update.transact(inshandle);
}


pub fn query() {
    let mut quiet = false;
    let mut explicit = false;

    let mut args = Arguments::new().prefix("-Q")
        .ignore("--query")
        .switch("-q", "--quiet", &mut quiet)
        .switch("-e", "--explicit", &mut explicit);

    args = args.parse_arguments();
    let targets = args.get_runtime().clone();

    if targets.len() < 1 {
        print_help_msg("Target not specified.");
    }
    
    query_database(targets.get(0).unwrap(), explicit, quiet) 
}

fn query_database(instance: &String, explicit: bool, quiet: bool) {    
    let instance_vars = InsVars::new(instance);

    test_root(&instance_vars);

    let root = instance_vars.root().as_str(); 
    let handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", instance_vars.root())).unwrap();

    for pkg in handle.localdb().pkgs() {
        if pkg.reason() != PackageReason::Explicit && explicit {
            continue;
        }

        if quiet {
            println!("{} ", pkg.name());
        } else {
            println!("{} {}", pkg.name(), style(pkg.version()).green().bold()); 
        }
    } 
}

pub fn instantiate_alpm(inshandle: &InstanceHandle) -> Alpm { 
    let root = inshandle.vars().root().as_str();  
    test_root(&inshandle.vars());
    let mut handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", root)).unwrap();
    handle.set_hookdirs(vec![format!("{}/etc/pacman.d/hooks/", root), format!("{}/usr/share/libalpm/hooks/", root)].iter()).unwrap();
    handle.set_cachedirs(vec![format!("{}/pkg", LOCATION.get_cache())].iter()).unwrap();
    handle.set_gpgdir(format!("{}/pacman/gnupg", LOCATION.get_data())).unwrap();
    handle.set_parallel_downloads(parallel_downloads());
    handle.set_logfile(format!("{}/pacwrap.log", LOCATION.get_data())).unwrap();
    handle.set_check_space(PACMAN_CONF.check_space);
    handle = register_remote(handle); handle
}

fn instantiate_alpm_syncdb(inshandle: &InstanceHandle) -> Alpm { 
    let root = inshandle.vars().root().as_str();  
    test_root(&inshandle.vars()); 
    let mut handle = Alpm::new2(root, &format!("{}/pacman/", LOCATION.get_data())).unwrap();
    handle.set_cachedirs(vec![format!("{}/pkg", LOCATION.get_cache())].iter()).unwrap();
    handle.set_progress_cb(ProgressCallback::new(), progress_event::progress_event);
    handle.set_gpgdir(format!("{}/pacman/gnupg", LOCATION.get_data())).unwrap(); 
    handle.set_dl_cb(DownloadCallback::new(0, 0), dl_event::download_event);
    handle.set_parallel_downloads(parallel_downloads());    
    handle = register_remote(handle); handle
}

fn register_remote(mut handle: Alpm) -> Alpm { 
    for repo in PACMAN_CONF.repos.iter() {
        let siglevel = signature(&repo.sig_level, *DEFAULT_SIGLEVEL); 
        let core = handle
        .register_syncdb_mut(repo.name.as_str(), siglevel)
        .unwrap();

        for server in repo.servers.iter() {
            core.add_server(server.as_str()).unwrap();
        }

        core.set_usage(Usage::ALL).unwrap();
    }

    handle
}

fn synchronize_database(cache: &InstanceCache, force: bool) {
     match cache.containers_base().get(0) {
        Some(insname) => {
            let ins: &InstanceHandle = cache.instances().get(insname).unwrap();      
            let mut handle = instantiate_alpm_syncdb(&ins);
    
            println!("{} {} ",style("::").bold().green(), style("Synchronising package databases...").bold()); 

            if let Err(err) = handle.syncdbs_mut().update(force) {
                print_error(format!("Unable to initialize transaction: {}.",err.to_string()));
                println!("{} Transaction failed.",style("->").bold().red());
                std::process::exit(1);
            }
            
            Alpm::release(handle).unwrap();  

            for i in cache.registered().iter() {
                let ins: &InstanceHandle = cache.instances().get(i).unwrap();
                let vars: &InsVars = ins.vars();
        
                for repo in PACMAN_CONF.repos.iter() {
                    let src = &format!("{}/pacman/sync/{}.db",constants::LOCATION.get_data(), repo.name);
                    let dest = &format!("{}/var/lib/pacman/sync/{}.db", vars.root(), repo.name);
                    if let Err(error) = linker::create_hard_link(src, dest) {
                        print_warning(error);
                    }
                }
            } 
        },
        None => {
            print_error("No compatible containers available to synchronize remote database.");
            exit(2)
        }
    }
}

fn signature(sigs: &Vec<String>, default: SigLevel) -> SigLevel {
    if sigs.len() > 0 {
        let mut sig = SigLevel::empty();

        for level in sigs {
            sig = sig | signature_level(level);
        }

        sig 
    } else {
        default
    }
}

fn signature_level(sig: &String) -> SigLevel {
    if sig == "PackageRequired" || sig == "PackageTrustedOnly" {
        return SigLevel::PACKAGE;
    } else if sig == "DatabaseRequired" || sig == "DatabaseTrustedOnly" {
        return SigLevel::DATABASE;
    } else if sig == "PackageOptional" {
        return SigLevel::PACKAGE_OPTIONAL;
    } else if sig == "PackageTrustAll" {
        return SigLevel::PACKAGE_UNKNOWN_OK | SigLevel::DATABASE_MARGINAL_OK;
    } else if sig == "DatabaseOptional" {
        return SigLevel::DATABASE_OPTIONAL;
    } else if sig == "DatabaseTrustAll" {
        return SigLevel::DATABASE_UNKNOWN_OK | SigLevel::PACKAGE_MARGINAL_OK;
    }

    SigLevel::empty()
}

fn parallel_downloads() -> u32 {
    match PACMAN_CONF.parallel_downloads.try_into() { Ok(i) => i, Err(_) => 1 }
}

fn invalid_environment() {
    print_error("Invalid environmental parameters.");
    exit(1);
}

fn validate_environment() {
    match std::env::var("LD_PRELOAD") {
        Ok(var) => {
            if var != "/usr/lib/libfakeroot/fakechroot/libfakechroot.so" {
                invalid_environment();
            }
        },
        Err(_) => invalid_environment()
    }
}
