use alpm::Package;
use alpm::Alpm;

pub fn format_unit(bytes: i64) -> String {
    let conditional: f64 = if bytes > -1 { 1000.0 } else { -1000.0 };
    let diviser: f64 = 1000.0;
    let mut size: f64 = bytes as f64;
    let mut idx: i8 = -1;

    while if bytes > -1 { size > conditional } else { size < conditional } {
        size = size / diviser;
        idx += 1;
    }
    
    if idx == -1 {
        format!("{:.0} {}", size, unit_suffix(idx))
    } else {
        format!("{:.2} {}", size, unit_suffix(idx)) 
    }
}

fn unit_suffix<'a>(i: i8) -> &'a str {
    match i {
        0 => "KB",
        1 => "MB",
        2 => "GB",
        3 => "TB",
        4 => "PB",
        _ => "B"
    }
}


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
                    .collect::<Vec<_>>().len() > 0 {
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
                        .collect::<Vec<_>>().len() > 0 {
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

pub fn usize_into_u64(u: usize) -> u64 {
    match u.try_into() { Ok(i) => i, Err(_) => { 0 }}
}

pub fn i32_into_u64(u: i32) -> u64 {
    match u.try_into() { Ok(i) => i, Err(_) => { 0 }}
}
