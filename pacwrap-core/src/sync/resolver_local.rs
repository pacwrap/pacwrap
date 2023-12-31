use std::collections::HashSet;

use alpm::{Package, Alpm, PackageReason};

use super::{transaction::{ErrorKind, TransactionType}, utils::AlpmUtils};

#[allow(dead_code)]
pub struct LocalDependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<Package<'a>>,
    ignored: &'a HashSet<String>,
    handle: &'a Alpm,
    depth: isize,
    flags: (bool, bool, bool),
} 

impl <'a>LocalDependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a HashSet<String>, trans_type: &TransactionType) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
            flags: match trans_type {
                TransactionType::Remove(enumerate, cascade, explicit) => (*enumerate, *cascade, *explicit),
                _ => panic!("Invalid transaction type for this resolver."),
            },
        }
    }

    fn check_depth(&mut self) -> Result<(), ErrorKind> {
        if self.depth == 50 {
            Err(ErrorKind::RecursionDepthExceeded(self.depth))?
        }
            
        self.depth += 1;
        Ok(())
    }
    
    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Result<Vec<Package<'a>>, ErrorKind> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        
        for pkg in packages {
            if let Some(_) = self.resolved.get(pkg) {
                continue;
            }

            if let Some(_) = self.ignored.get(*pkg) {
                continue;
            }

            if let Some(pkg) = self.handle.get_local_package(pkg) {    
                if self.depth > 0 {
                    //TODO: Implement proper explicit package handling
                    if ! self.flags.1
                    && pkg.reason() == PackageReason::Explicit {
                        continue;
                    }
 
                    if pkg.required_by()
                        .iter()
                        .filter_map(|p|
                        match self.resolved.get(p) {
                            None => Some(()), Some(_) => None
                        })
                        .count() > 0 {
                        continue;
                    }
                }

                self.packages.push(pkg);
                self.resolved.insert(pkg.name());
                
                if ! self.flags.0 {
                    continue;
                }

                synchronize.extend(pkg.depends()
                    .iter()
                    .map(|pkg| pkg.name())
                    .collect::<Vec<&str>>());

                if ! self.flags.1 {
                    continue;
                }

                for package in self.handle.localdb().pkgs() { 
                    if package.depends()
                        .iter()
                        .filter_map(|d| self.resolved.get(d.name()))
                        .count() > 0 {
                        synchronize.push(package.name());
                    }
                }
            }             
        }

        if synchronize.len() > 0 && self.flags.0 {
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
            Ok(self.packages)
        }
    }
}
