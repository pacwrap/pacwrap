#![allow(dead_code)]

use alpm::{Alpm, PackageReason, Package, TransFlag};
use console::style;

use crate::sync;
use crate::sync::dl_event;
use crate::sync::dl_event::DownloadCallback;
use crate::sync::progress_event;
use crate::sync::progress_event::ProgressCallback;
use crate::utils::prompt::prompt;
use crate::config::cache::InstanceCache;
use crate::config::InstanceHandle;


pub struct Update {
    queried: Vec<String>,
    updated: Vec<String>,
}

impl Update {
    
    pub fn new() -> Self {
        Self {
            queried: Vec::new(),
            updated: Vec::new(),
        }  
    }

    pub fn updated(&self) -> &Vec<String> {
        &self.updated
    }

    pub fn update(&mut self, cache: &InstanceCache, containers: &Vec<String>) {
        for ins in containers.iter() { 
            if self.queried.contains(ins) {
                continue;
            }

            let inshandle = cache.instances().get(ins).unwrap();
       
            self.update(cache, inshandle.instance().dependencies()); 
            self.update_instance(&cache, inshandle, false);  
            self.queried.push(ins.clone());
        }
    }

    fn update_instance(&mut self, cache: &InstanceCache, inshandle: &InstanceHandle, dbonly: bool) {
        if ! dbonly {
            println!("{} {}",style("::").bold().cyan(), style(format!("Checking {} for updates...", inshandle.vars().instance())).bold());   
        } else {
            println!("{} {}",style("->").bold().cyan(), style(format!("Synchronizing foreign packages")));    
        }

        let mut handle = sync::instantiate_alpm(&inshandle);      
        let mut local: Vec<Package> = Vec::new();
        let mut remote: Vec<Package> = Vec::new();
        let mut flags = TransFlag::NO_DEP_VERSION; 
        let mut ignored: Vec<String> = Vec::new();
        let config = inshandle.instance();
        let deps = config.dependencies();
        let dep_depth = deps.len(); 
        let instance = inshandle.vars().instance();

        if dep_depth > 0 {
            if dbonly {
                flags = flags | TransFlag::DB_ONLY;
        
                for pkg in handle.localdb().pkgs() {
                    if pkg.reason() != PackageReason::Explicit {
                        continue;
                    }
        
                    ignored.push(pkg.name().into()); 
                }
            } else {
                let dep_handle = sync::instantiate_alpm(cache.instances().get(&deps[dep_depth-1]).unwrap());

                for pkg in dep_handle.localdb().pkgs() {
                    ignored.push(pkg.name().into()); 
                }
            
                Alpm::release(dep_handle).unwrap();
            }
        
            for ignore in ignored.iter() {
                handle.add_ignorepkg(ignore.as_bytes()).unwrap();
            }
        } else if dbonly {
            Alpm::release(handle).unwrap(); 
            return;
        }

        for pkg in handle.localdb().pkgs() {
            for syncdb in handle.syncdbs() { 
                if let Ok(pkg_remote) = syncdb.pkg(pkg.name()) { 
                    if pkg_remote.version() > pkg.version() && ! ignored.contains(&pkg.name().into()) {
                        local.push(pkg);
                        remote.push(pkg_remote);
                    } 
                }   
            }
        }

        if local.len() > 0 {
            if ! dbonly { 
                self.update_instance(&cache, inshandle, true); 
                println!("{} {} \n",style("::").bold().red(), style("Package changes").bold());
        
                for val in 0..local.len() { 
                    let pkg_remote = remote[val];
                    let pkg = local[val];
                    println!("{} {} -> {}", style(pkg.name()).bold(), style(pkg.version()).bold().yellow(), style(pkg_remote.version()).bold().green());
                } println!(); 
         
                if let Err(_) = prompt("Proceed with installation?") {
                    return;
                }
                handle.set_progress_cb(ProgressCallback::new(true), progress_event::progress_event);
                handle.set_dl_cb(DownloadCallback::new(true), dl_event::download_event);   
            }

            handle.trans_init(flags).unwrap();  
            for val in 0..local.len() { 
                handle.trans_add_pkg(remote[val]).unwrap();
            }

            handle.trans_prepare().unwrap();
            handle.trans_commit().unwrap();
            handle.trans_release().unwrap();

            self.updated.push(instance.clone());
        } else if ! dbonly {
            println!("{} {} is up-to-date!", style("->").bold().green(), instance);
        }

        Alpm::release(handle).unwrap(); 
    }
}
