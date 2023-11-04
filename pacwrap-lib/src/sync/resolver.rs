use std::{rc::Rc, collections::HashSet};

use alpm::{Package, Alpm};

use super::{transaction::Error,
    utils::{get_package, get_local_package}};

pub struct DependencyResolver<'a> {
    resolved: HashSet<&'a str>,
    packages: Vec<Package<'a>>,
    keys: Vec<Rc<str>>,
    ignored: &'a HashSet<&'a str>,
    handle: &'a Alpm,
    depth: isize,
} 

impl <'a>DependencyResolver<'a> {
    pub fn new(alpm: &'a Alpm, ignorelist: &'a HashSet<&'a str>) -> Self {
        Self {
            resolved: HashSet::new(),
            packages: Vec::new(),
            keys: Vec::new(),
            ignored: ignorelist,
            depth: 0,
            handle: alpm,
        }
    }

    fn check_depth(&mut self) -> Result<(), Error> {
        if self.depth == 50 { 
            Err(Error::RecursionDepthExceeded(self.depth))?
        }

        self.depth += 1;
        Ok(())
    }
    
    pub fn enumerate(mut self, packages: &Vec<&'a str>) -> Result<(Vec<Rc<str>>, Vec<Package<'a>>), Error> {
        let mut synchronize: Vec<&'a str> = Vec::new(); 
        
        for pkg in packages {
            if let Some(_) = self.resolved.get(pkg) {
                continue;
            }

            if let Some(_) = self.ignored.get(pkg) {
                continue;
            }

            if let Some(pkg) = get_package(&self.handle, pkg) {   
                self.packages.push(pkg);
                self.resolved.insert(pkg.name());
                synchronize.extend(pkg.depends()
                    .iter()
                    .filter_map(|p| 
                        match get_local_package(&self.handle, p.name()) {
                            None => match get_package(&self.handle, p.name()) {  
                                Some(dep) => Some(dep.name()), None => None,
                            },
                            Some(_) => None,
                        }
                    )
                    .collect::<Vec<&str>>());

                if self.depth > 0 {
                    self.keys.push(pkg.name().into());
                }
            }             
        }

        if synchronize.len() > 0 { 
            self.check_depth()?;
            self.enumerate(&synchronize)
        } else {
           Ok((self.keys, self.packages))
        }
    }
}
