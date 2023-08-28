use std::process::exit;
use std::rc::Rc;

use alpm::{Package, Alpm};

use crate::sync::utils::{get_package, get_local_package};
use crate::utils::print_error;

pub struct DependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    keys: Vec<Rc<str>>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: i8,
} 

impl <'a>DependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a Vec<&'a str>) -> Self {
        Self {
            resolved: Vec::new(),
            packages: Vec::new(),
            keys: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
        }
    }

    fn check_depth(&mut self) {
        if self.depth == 50 {
            print_error("Recursion depth exceeded maximum.");
            exit(2);
        }
    }
    
    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> (Vec<Rc<str>>, Vec<Package<'a>>) {
        let mut synchronize: Vec<&'a str> = Vec::new();
        self.check_depth();

        for pkg in packages {
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            } 

            if let Some(pkg) = get_package(&self.handle, pkg) {  
                let deps = pkg.depends()
                    .iter()
                    .map(|p| p.name())
                    .collect::<Vec<&str>>();

                self.resolved.push(pkg.name());
                self.packages.push(pkg);
                
                if self.depth > 0 {
                    self.keys.push(pkg.name().into());
                }

                for dep in deps {
                    if let None = get_local_package(&self.handle, dep) {
                        if let Some(dep) = get_package(&self.handle, dep) {  
                            synchronize.push(dep.name());
                        }
                    }
                }
            }             
        }

        if synchronize.len() > 0 {
            self.depth += 1;
            self.enumerate(&synchronize)
        } else {
            (self.keys, self.packages)
        }
    }
}
