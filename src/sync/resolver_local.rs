use std::process::exit;

use alpm::{Package, Alpm};

use crate::sync::utils::get_local_package;
use crate::utils::print_error;

pub struct LocalDependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: i8,
    recursive: bool,
    cascading: bool,
} 

impl <'a>LocalDependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a Vec<&'a str>, recurse: bool, cascade: bool) -> Self {
        Self {
            resolved: Vec::new(),
            packages: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
            recursive: recurse,
            cascading: cascade
        }
    }

    fn check_depth(&mut self) {
        if self.depth == 50 {
            print_error("Recursion depth exceeded maximum.");
            exit(2);
        }
    }
    
    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Vec<Package<'a>> {
        let mut synchronize: Vec<&'a str> = Vec::new();
        self.check_depth();

        for pkg in packages {
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            }

            if let Some(pkg) = get_local_package(&self.handle, pkg) {  
                if self.depth > 0 {
                    let required = pkg.required_by();
                    let mut skip = false;

                    for req in required {  
                        if self.resolved.contains(&req.as_str()) {
                            continue;
                        }

                        skip = true;
                        break;
                    }

                    if skip {
                        continue;
                    }
                }

                let deps = pkg.depends().iter().map(|p| p.name()).collect::<Vec<&str>>();
                let deps_opt = pkg.optdepends().iter().map(|p| p.name()).collect::<Vec<&str>>();

                for dep in deps { 
                    if let Some(dep) = get_local_package(&self.handle, dep) {  
                        synchronize.push(dep.name());
                    }
                }

                for dep in deps_opt {
                    if let Some(dep) = get_local_package(&self.handle, dep) {  
                        synchronize.push(dep.name());
                    }
                }

                self.resolved.push(pkg.name());
                self.packages.push(pkg);

                if ! self.cascading {
                    continue;
                }

                for package in self.handle.localdb().pkgs() { 
                    for dep in package.depends() {
                        if ! self.resolved.contains(&dep.name()) {
                            continue;
                        }
                      
                        synchronize.push(package.name());
                    }
                }
            }             
        }

        if synchronize.len() > 0 && self.recursive {
            self.depth += 1;       
            self.enumerate(&synchronize)
        } else {
            self.packages
        }
    }
}
