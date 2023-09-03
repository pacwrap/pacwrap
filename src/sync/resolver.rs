use std::rc::Rc;

use alpm::{Package, Alpm};

use super::{transaction::Error,
    utils::{get_package, get_local_package}};

pub struct DependencyResolver<'a> {
    resolved: Vec<&'a str>,
    packages: Vec<Package<'a>>,
    keys: Vec<Rc<str>>,
    ignored: &'a Vec<&'a str>,
    handle: &'a Alpm,
    depth: isize,
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
            if self.resolved.contains(&pkg) || self.ignored.contains(&pkg) {
                continue;
            } 

            if let Some(pkg) = get_package(&self.handle, pkg) {   
                self.resolved.push(pkg.name());
                self.packages.push(pkg);               
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
