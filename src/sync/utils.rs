use alpm::Package;
use alpm::Alpm;

pub fn get_local_package<'a>(handle: &'a Alpm, pkg: &'a str) -> Option<Package<'a>> {
    if let Ok(pkg) = handle.localdb().pkg(pkg) {
        return Some(pkg);
    } else {
        handle.localdb()
            .pkgs()
            .iter()
            .find_map(|f| {
            if f.provides()
                    .iter()
                    .filter(|d| pkg == d.name())
                    .count() > 0 {
                Some(f)
            } else {
                None
            }  
        })
    }
}

pub fn get_package<'a>(handle: &'a Alpm, pkg: &'a str) -> Option<Package<'a>> {
    for sync in handle.syncdbs() {  
        if let Ok(pkg) = sync.pkg(pkg) {
           return Some(pkg);
        } else {
            let package = sync.pkgs()
                .iter()
                .find_map(|f| {
                if f.provides()
                        .iter()
                        .filter(|d| pkg == d.name())
                        .count() > 0 {
                    Some(f)
                } else {
                    None
                }  
            });

            if let None = package {
                continue;
            }

            return package
        }
    }

    None
}

pub fn whitespace(total: usize, current: usize) -> String {
    let difference = total-current;
    let mut whitespace = String::new();
    if difference > 0 {
        for _ in 0..difference {
            whitespace.push_str(" ");
        } 
    }

    whitespace
}
