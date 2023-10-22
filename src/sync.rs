use std::fs::File;
use std::io::Write;
use std::process::{exit, Command};
use std::env;

use alpm::{Alpm,  SigLevel, Usage, PackageReason};
use console::style;
use lazy_static::lazy_static;
use pacmanconf;

use crate::config::{Instance, InstanceType, self};
use crate::constants::{self, LOCATION};
use crate::log::Logger;
use crate::sync::{
    dl_event::DownloadCallback,
    progress_event::ProgressCallback,
    transaction::TransactionType,
    transaction::aggregator};
use crate::utils::arguments::Operand;
use crate::utils::{arguments::Arguments, 
    test_root,
    handle_process,
    print_warning,
    print_error,
    print_help_error};
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

pub fn synchronize(mut args: Arguments) {
    if let Err(_) = validate_environment() {
        print_error("Execution without libfakechroot in an unprivileged context is not supported.");
        exit(1);
    }

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

    if let Some(instype) = create_type(&mut args) {
        if let TransactionType::Upgrade(upgrade, refresh, _) = action { 
            if ! upgrade {
                print_help_error("--upgrade/-u not supplied with --create/-c.");
            } else if ! refresh {
                print_help_error("--refresh/-y not supplied with --create/-c.");
            }
        }

        create(instype, args.targets());
    }

    aggregator::upgrade(action, &mut args, &mut cache, &mut logger).aggregate(&mut InstanceCache::new());
}

pub fn remove(mut args: Arguments) {
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
    
    aggregator::remove(action, &mut args, &mut cache, &mut logger).aggregate(&mut InstanceCache::new());
}

fn create_type(args: &mut Arguments) -> Option<InstanceType> {
    let mut instype = None;
    let mut create = false;

    args.set_index(1);

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('c') | Operand::Long("create") => create = true, 
            Operand::Short('b') | Operand::Long("base") => instype = Some(InstanceType::BASE),
            Operand::Short('d') | Operand::Long("slice") => instype = Some(InstanceType::DEP),
            Operand::Short('r') | Operand::Long("root") => instype = Some(InstanceType::ROOT),
            _ => continue,
        } 
    }

    if create { instype } else { None }
}

pub fn create(instype: InstanceType, mut targets: Vec<&str>) {
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
    println!("{} {}", style("::").bold().green(), style(format!("Instantiating container {}...", ins)).bold());

    let deps = deps.iter().map(|a| { let a = *a; a.into() }).collect();
    let mut logger = Logger::new("pacwrap").init().unwrap();
    let instance = match config::provide_some_handle(ins) {
        Some(mut handle) => {
            handle.metadata_mut().set(deps, vec!());
            handle
        },
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

pub fn query(mut arguments: Arguments) {
    let mut target = "";
    let mut explicit = false;
    let mut quiet = false;

    while let Some(arg) = arguments.next() {
        match arg {
            Operand::Short('e') | Operand::Long("explicit") => explicit = true,
            Operand::Short('q') | Operand::Long("quiet") => quiet = true,
            Operand::LongPos("target", t) | Operand::ShortPos(_, t) => target = t,
            _ => arguments.invalid_operand(),
        }
    }

    if target.is_empty() {
        print_help_error("Target not specified.");
    }

    let insvars = InsVars::new(target);

    test_root(&insvars);

    let root = insvars.root().as_ref(); 
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
