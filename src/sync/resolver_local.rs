
use alpm::{Package, Alpm, PackageReason};

use crate::sync::utils::get_local_package;

use super::transaction::Error;

pub struct LocalDependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: isize,
    recursive: bool,
    cascading: bool,
    explicit: bool,
} 

impl <'a>LocalDependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a Vec<&'a str>, recurse: bool, cascade: bool, exp: bool) -> Self {
        Self {
            resolved: Vec::new(),
            packages: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
            recursive: recurse,
            cascading: cascade,
            explicit: exp,
        }
    }

    fn check_depth(&mut self) -> Result<(), Error> {
        if self.depth == 50 {
            Err(Error::RecursionDepthExceeded(self.depth))?
        }
            
        self.depth += 1;
        Ok(())
    }
    
    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Result<Vec<Package<'a>>, Error> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        
        for pkg in packages {
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            }

            if let Some(pkg) = get_local_package(&self.handle, pkg) {   
                let req_by = pkg.required_by();
                let required = req_by.iter().collect::<Vec<&str>>();

                if required.iter().filter_map(|p|
                    match self.resolved.contains(&p) {
                        false => Some(()), true => None
                    }).collect::<Vec<_>>().len() > 0 {
                    continue;
                }

                if self.explicit && self.depth > 0
                && pkg.reason() == PackageReason::Explicit {
                    continue;
                }

                self.resolved.push(pkg.name());
                self.packages.push(pkg);
                synchronize.extend(pkg.depends()
                    .iter()
                    .filter_map(|p| 
                    match get_local_package(&self.handle, p.name()) { 
                        Some(pkg) => Some(pkg.name()), None => None, 
                    })
                    .collect::<Vec<&str>>());

                if ! self.cascading {
                    continue;
                }

                for package in self.handle.localdb().pkgs() { 
                    if package.depends().iter().filter_map(|d| 
                        match self.resolved.contains(&d.name()) {
                            true => Some(()), false => None
                        }).collect::<Vec<_>>().len() > 0 {
                        synchronize.push(package.name());
                    }
                }
            }             
        }

        if synchronize.len() > 0 && self.recursive {
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
            Ok(self.packages)
        }
    }
}
