use std::fs::File;
use std::io::Write;
use std::process::{exit, Command};
use std::env;
use std::rc::Rc;

use alpm::{Alpm,  SigLevel, Usage, PackageReason};
use console::style;
use lazy_static::lazy_static;
use pacmanconf;

use crate::config::{Instance, InstanceType, self};
use crate::constants::{self, LOCATION};
use crate::log::Logger;
use crate::sync::{
    dl_event::DownloadCallback,
    filesystem::FileSystemStateSync,
    progress_event::ProgressCallback,
    transaction::TransactionType,
    transaction::TransactionAggregator};
use crate::utils::{print_help_error, print_help_msg};
use crate::utils::{Arguments, 
    arguments::invalid, 
    test_root,
    handle_process,
    print_warning,
    print_error};
use crate::config::{InsVars,
    InstanceHandle,
    cache::InstanceCache};

lazy_static! {
    static ref PACMAN_CONF: pacmanconf::Config = pacmanconf::Config::from_file(format!("{}/pacman.conf", constants::LOCATION.get_config())).unwrap(); 
    static ref DEFAULT_SIGLEVEL: SigLevel = signature(&PACMAN_CONF.sig_level, SigLevel::PACKAGE | SigLevel::DATABASE_OPTIONAL);
}

mod progress_event;
mod dl_event;
mod query_event;
mod filesystem;
mod transaction;
mod resolver;
mod resolver_local;
mod utils;

pub fn synchronize() {
    if let Err(_) = validate_environment() {
        print_error("Execution without libfakechroot in an unprivileged context is not supported.");
        exit(1);
    }

    let mut base = false;
    let mut dep = false;
    let mut create = false;
    let mut force_database = false;
    let mut refresh = false;
    let mut upgrade = false;
    let mut preview = false;
    let mut no_confirm = false;
    let mut no_deps = false;
    let mut dbonly = false;
    let mut y_count = 0;
    let mut u_count = 0;
    let args = Arguments::new()
        .prefix("-S")
        .ignore("--sync")
        .ignore("--fake-chroot")
        .short("-y").long("--refresh").map(&mut refresh).set(true).count(&mut y_count).push()
        .short("-u").long("--upgrade").map(&mut upgrade).set(true).count(&mut u_count).push()
        .short("-p").long("--preview").map(&mut preview).set(true).push()
        .short("-o").long("--target-only").map(&mut no_deps).set(true).push()
        .short("-c").long("--create").map(&mut create).set(true).push() 
        .short("-d").long("--dep").map(&mut dep).set(true).push() 
        .short("-b").long("--base").map(&mut base).set(true).push() 
        .long("--force-foreign").map(&mut force_database).set(true).push()
        .long("--db-only").map(&mut dbonly).set(true).push()
        .long("--noconfirm").map(&mut no_confirm).set(true).push() 
        .parse_arguments();
    let mut cache = InstanceCache::new();
    let targets = args.targets().clone();
    let mut runtime = args.get_runtime().clone();

    if create {
        if ! upgrade {
            print_help_msg("--upgrade is required for --create.");
            exit(1); 
        } else if ! refresh {
            print_help_msg("--refresh is required for --create."); 
        }

        if targets.len() == 0 {
            print_help_error("Target required for --create.");
            exit(1);
        }

        let instype = if base {
            InstanceType::BASE
        } else if dep {
            InstanceType::DEP
        } else {
            InstanceType::ROOT
        };
        let ins = targets.get(0)
            .unwrap()
            .as_ref();
        let deps = targets.iter()
            .filter_map(|p| {
            if p.as_ref() != ins {
                Some(p.as_ref().into())
            } else {
                None
            } 
        }).collect::<Vec<Rc<str>>>();

        if base {
            runtime.extend(vec!("base".into(), "pacwrap-base-dist".into()));
        }

        instantiate_container(ins, deps, instype);
    }

    if targets.len() > 0 {
        cache.populate_from(&targets, true);
    } else {
        cache.populate();
    }

    if y_count == 4 {
        let mut l: FileSystemStateSync = FileSystemStateSync::new(&cache);  

        l.prepare(cache.registered().len());
        l.engage(&cache.registered());
        l.finish();
    } else if upgrade || targets.len() > 0 {
        if refresh {
            synchronize_database(&cache, y_count == 2); 
        }

        let transaction_type = TransactionType::Upgrade(upgrade, u_count > 1);
        let mut logger = Logger::new("pacwrap-sync").init().unwrap();
        let mut update = TransactionAggregator::new(transaction_type, &cache, &mut logger)
            .preview(preview)
            .force_database(force_database || create)
            .database_only(y_count > 2 || dbonly)
            .no_confirm(no_confirm)
            .create(create);
           
        if targets.len() > 0 { 
            let target = targets.get(0).unwrap();
            let inshandle = cache.instances().get(target).unwrap();

            update.queue(target.clone(), runtime);
            
            if no_deps || ! upgrade {
                update.transact(&inshandle);
            } else {
                transaction::update(update, &cache, &mut InstanceCache::new(), create); 
            }
        } else {
            transaction::update(update, &cache, &mut InstanceCache::new(), create);
        }
    } else if refresh {
        synchronize_database(&cache, y_count == 2); 
    } else {
        invalid();
    }
}

fn instantiate_container(ins: &str, deps: Vec<Rc<str>>, instype: InstanceType) {
    if let InstanceType::ROOT | InstanceType::DEP = instype {
        if deps.len() == 0 {
            print_help_error("Dependencies are required for creation of root and dependent (sliced) filesystems.");
            exit(1);
        }
    }

    println!("{} {}", style("::").bold().green(), style(format!("Instantiating container {}...", ins)).bold());

    let mut logger = Logger::new("pacwrap").init().unwrap();
    let instance = match config::provide_some_handle(ins) {
        Some(handle) => handle,
        None => {
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
    println!("{} Instantiation complete.", style("->").bold().green());
}

pub fn remove() { 
    let mut preview = false;
    let mut recursive = false;
    let mut cascade = false;
    let mut no_confirm = false;
    let mut db_only = false;
    let mut recursive_count = 0;
    let args = Arguments::new()
        .prefix("-R")
        .ignore("--remove")
        .short("-p").long("--preview").map(&mut preview).set(true).push()
        .short("-s").long("--recursive").map(&mut recursive).set(true).count(&mut recursive_count).push()
        .short("-c").long("--cascade").map(&mut cascade).set(true).push()
        .long("--db-only").map(&mut db_only).set(true).push()
        .long("--noconfirm").map(&mut no_confirm).set(true)
        .parse_arguments()
        .require_target(1);
    let mut targets = args.targets().clone();
    let runtime = args.get_runtime().clone();
    let mut cache: InstanceCache = InstanceCache::new();
   
    cache.populate_from(&targets, true);

    let target = targets.remove(0);
    let inshandle = cache.instances().get(&target).unwrap();
    let transaction_type = TransactionType::Remove(recursive, cascade, recursive_count < 2); 
    let mut logger = Logger::new("pacwrap-sync").init().unwrap();
    let mut update = TransactionAggregator::new(transaction_type, &cache, &mut logger)
        .preview(preview)
        .database_only(db_only)
        .no_confirm(no_confirm);

    update.queue(target, runtime);
    update.transact(&inshandle);
}

pub fn query() {
    let mut quiet = false;
    let mut explicit = false;
    let args = Arguments::new().prefix("-Q")
        .ignore("--query")
        .short("-q").long("--quiet").map(&mut quiet).set(true).push()
        .short("-e").long("--explicit").map(&mut explicit).set(true).push()
        .assume_target()
        .parse_arguments()
        .require_target(1);
    let targets = args.targets().clone();
    let target = targets.get(0).unwrap().as_ref();
    let instance_vars = InsVars::new(target);

    test_root(&instance_vars);
    query_database(instance_vars, explicit, quiet) 
}

fn query_database(vars: InsVars, explicit: bool, quiet: bool) {    
    let root = vars.root().as_ref(); 
    let handle = Alpm::new2(root, &format!("{}/var/lib/pacman/", root)).unwrap();

    for pkg in handle.localdb().pkgs() {
        if explicit && pkg.reason() != PackageReason::Explicit {
            continue;
        }

        match quiet {
            true => println!("{} ", pkg.name()),
            false => println!("{} {} ", pkg.name(), style(pkg.version()).green().bold()), 
        }
    } 
}

pub fn interpose() {
    let arguments = env::args().skip(1).collect::<Vec<_>>(); 
    let all_args = env::args().collect::<Vec<_>>();
    let this_executable = all_args.first().unwrap();

    handle_process(Command::new(this_executable)
        .env("LD_PRELOAD", "/usr/lib/libfakeroot/fakechroot/libfakechroot.so")
        .arg("--fake-chroot")
        .args(arguments)
        .spawn());
}

pub fn instantiate_alpm(inshandle: &InstanceHandle) -> Alpm { 
    alpm_handle(inshandle, format!("{}/var/lib/pacman/", inshandle.vars().root()))
}

fn alpm_handle(inshandle: &InstanceHandle, db_path: String) -> Alpm { 
    test_root(&inshandle.vars());

    let root = inshandle.vars().root().as_ref();   
    let mut handle = Alpm::new(root, &db_path).unwrap();

    handle.set_hookdirs(vec![format!("{}/usr/share/libalpm/hooks/", root), format!("{}/etc/pacman.d/hooks/", root)].iter()).unwrap();
    handle.set_cachedirs(vec![format!("{}/pkg", LOCATION.get_cache())].iter()).unwrap();
    handle.set_gpgdir(format!("{}/pacman/gnupg", LOCATION.get_data())).unwrap();
    handle.set_parallel_downloads(PACMAN_CONF.parallel_downloads.try_into().unwrap_or(1));
    handle.set_logfile(format!("{}/pacwrap.log", LOCATION.get_data())).unwrap();
    handle.set_check_space(PACMAN_CONF.check_space);
    handle = register_remote(handle); 
    handle
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

    let siglevel = signature(&vec!(format!("DatabaseTrustAll")), SigLevel::DATABASE_OPTIONAL); 
    let core = handle
        .register_syncdb_mut("pacwrap", siglevel)
        .unwrap();

    core.add_server(env!("PACWRAP_DIST_REPO")).unwrap();
    core.set_usage(Usage::ALL).unwrap();

    handle
}

fn synchronize_database(cache: &InstanceCache, force: bool) {
     match cache.containers_base().get(0) {
        Some(insname) => {
            let db_path = format!("{}/pacman/", constants::LOCATION.get_data());
            let ins: &InstanceHandle = cache.instances().get(insname).unwrap();      
            let mut handle = alpm_handle(&ins, db_path);

            println!("{} {} ",style("::").bold().green(), style("Synchronising package databases...").bold()); 
            handle.set_progress_cb(ProgressCallback::new(), progress_event::progress_event); 
            handle.set_dl_cb(DownloadCallback::new(0, 0), dl_event::download_event);

            if let Err(err) = handle.syncdbs_mut().update(force) {
                print_error(format!("Unable to initialize transaction: {}.",err.to_string()));
                println!("{} Transaction failed.",style("->").bold().red());
                std::process::exit(1);
            }
           
            Alpm::release(handle).unwrap();  

            for i in cache.registered().iter() {
                let ins: &InstanceHandle = cache.instances().get(i).unwrap();
                let vars: &InsVars = ins.vars();
                let src = &format!("{}/pacman/sync/{}.db",constants::LOCATION.get_data(), "pacwrap");
                let dest = &format!("{}/var/lib/pacman/sync/{}.db", vars.root(), "pacwrap");
                
                if let Err(error) = filesystem::create_hard_link(src, dest) {
                     print_warning(error);
                }

                for repo in PACMAN_CONF.repos.iter() {
                    let src = &format!("{}/pacman/sync/{}.db",constants::LOCATION.get_data(), repo.name);
                    let dest = &format!("{}/var/lib/pacman/sync/{}.db", vars.root(), repo.name);
                    if let Err(error) = filesystem::create_hard_link(src, dest) {
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
            sig = sig | if level == "PackageRequired" || level == "PackageTrustedOnly" {
                SigLevel::PACKAGE
            } else if level == "DatabaseRequired" || level == "DatabaseTrustedOnly" {
                SigLevel::DATABASE
            } else if level == "PackageOptional" {
                SigLevel::PACKAGE_OPTIONAL
            } else if level == "PackageTrustAll" {
                SigLevel::PACKAGE_UNKNOWN_OK | SigLevel::DATABASE_MARGINAL_OK
            } else if level == "DatabaseOptional" {
                SigLevel::DATABASE_OPTIONAL
            } else if level == "DatabaseTrustAll" {
                SigLevel::DATABASE_UNKNOWN_OK | SigLevel::PACKAGE_MARGINAL_OK
            } else {
                SigLevel::empty()
            }
        }

        sig 
    } else {
        default
    }
}

fn validate_environment() -> Result<(),()> {
    match std::env::var("LD_PRELOAD") {
        Ok(var) => {
            if var != "/usr/lib/libfakeroot/fakechroot/libfakechroot.so" {
                Err(())?
            }

            Ok(())
        },
        Err(_) => Err(())
    }
}
