use std::process::exit;

use alpm::{Alpm,  SigLevel, Usage};
use lazy_static::lazy_static;
use pacmanconf;
use serde::{Serialize, Deserialize};

use crate::constants::{BAR_GREEN, RESET, BOLD, ARROW_RED, CACHE_DIR, DATA_DIR, CONFIG_DIR};
use crate::sync::dl_event::DownloadCallback;
use crate::utils::{print_warning, print_error};
use crate::config::{InsVars,
    InstanceHandle,
    cache::InstanceCache};

lazy_static! {
    static ref PACMAN_CONF: pacmanconf::Config = pacmanconf::Config::from_file(format!("{}/pacman.conf", *CONFIG_DIR)).unwrap(); 
    static ref DEFAULT_SIGLEVEL: SigLevel = signature(&PACMAN_CONF.sig_level, SigLevel::PACKAGE | SigLevel::DATABASE_OPTIONAL);
    static ref DEFAULT_ALPM_CONF: AlpmConfigData = AlpmConfigData::new();
}

pub mod progress_event;
pub mod dl_event;
pub mod query_event;
pub mod utils;
pub mod transaction;
pub mod filesystem;
mod resolver;
mod resolver_local;

#[derive(Serialize, Deserialize)]
pub struct AlpmConfigData {
    repos: Vec<(String, u32, Vec<String>)>,
}

impl AlpmConfigData {
    fn new() -> Self {
        let mut remotes = Vec::new();

        for repo in PACMAN_CONF.repos.iter() {
            remotes.push((repo.name.clone(), signature(&repo.sig_level, *DEFAULT_SIGLEVEL).bits(), repo.servers.clone()));
        }

        remotes.push(("pacwrap".into(), 
            (SigLevel::PACKAGE_MARGINAL_OK | SigLevel::DATABASE_MARGINAL_OK).bits(), 
            vec![format!("file://{}", env!("PACWRAP_DIST_REPO")), format!("file:///tmp/dist-repo/")]));
 
        Self {
            repos: remotes,
        }
    }
}

pub fn instantiate_alpm_agent(remotes: &AlpmConfigData) -> Alpm {
    let mut handle = Alpm::new("/mnt/", "/mnt/var/lib/pacman/").unwrap();

    handle.set_logfile("/tmp/pacwrap.log").unwrap(); 
    handle.set_hookdirs(vec!["/mnt/usr/share/libalpm/hooks/", "/mnt/etc/pacman.d/hooks/"].iter()).unwrap();
    handle.set_cachedirs(vec!["/tmp/pacman/pkg"].iter()).unwrap();
    handle.set_gpgdir("/tmp/pacman/gnupg").unwrap();
    handle.set_parallel_downloads(5);
    handle.set_logfile("/tmp/pacwrap.log").unwrap();
    handle.set_check_space(false);
    handle = register_remote(handle, remotes); 
    handle
}

pub fn instantiate_alpm(inshandle: &InstanceHandle) -> Alpm { 
    alpm_handle(inshandle.vars(), format!("{}/var/lib/pacman/", inshandle.vars().root()), &*DEFAULT_ALPM_CONF)

}

fn alpm_handle(insvars: &InsVars, db_path: String, remotes: &AlpmConfigData) -> Alpm { 
    let root = insvars.root();
    let mut handle = Alpm::new(root, &db_path).unwrap();

    handle.set_cachedirs(vec![format!("{}/pkg", *CACHE_DIR)].iter()).unwrap();
    handle.set_gpgdir(format!("{}/pacman/gnupg", *DATA_DIR)).unwrap();
    handle.set_parallel_downloads(PACMAN_CONF.parallel_downloads.try_into().unwrap_or(1));
    handle.set_logfile(format!("{}/pacwrap.log", *DATA_DIR)).unwrap();
    handle.set_check_space(PACMAN_CONF.check_space);
    handle = register_remote(handle, remotes); 
    handle
}

fn register_remote(mut handle: Alpm, config: &AlpmConfigData) -> Alpm { 
    for repo in &config.repos {
        let core = handle.register_syncdb_mut(repo.0.clone(), 
            SigLevel::from_bits(repo.1).unwrap()).unwrap();

        for server in &repo.2 {
            core.add_server(server.as_str()).unwrap();
        }

        core.set_usage(Usage::ALL).unwrap();
    }

    handle
}

fn synchronize_database(cache: &InstanceCache, force: bool) {
     match cache.obtain_base_handle() {
        Some(ins) => {
            let db_path = format!("{}/pacman/", *DATA_DIR);
            let mut handle = alpm_handle(&ins.vars(), db_path, &*DEFAULT_ALPM_CONF);

            println!("{} {}Synchronising package databases...{}", *BAR_GREEN, *BOLD, *RESET); 
            handle.set_dl_cb(DownloadCallback::new(0, 0), dl_event::download_event);

            if let Err(err) = handle.syncdbs_mut().update(force) {
                print_error(format!("Unable to initialize transaction: {}.",err.to_string()));
                println!("{} Transaction failed.", *ARROW_RED);
                std::process::exit(1);
            }
           
            Alpm::release(handle).unwrap();  

            for i in cache.registered().iter() {
                let ins: &InstanceHandle = cache.get_instance(i).unwrap();
                let vars: &InsVars = ins.vars();
                let src = &format!("{}/pacman/sync/{}.db",*DATA_DIR, "pacwrap");
                let dest = &format!("{}/var/lib/pacman/sync/{}.db", vars.root(), "pacwrap");
                
                if let Err(error) = filesystem::create_hard_link(src, dest) {
                     print_warning(error);
                }

                for repo in PACMAN_CONF.repos.iter() {
                    let src = &format!("{}/pacman/sync/{}.db",*DATA_DIR, repo.name);
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
