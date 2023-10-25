use std::process::Command;
use std::env::var;

fn head() -> String {
    match Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
        Ok(output) =>  String::from_utf8(output.stdout).unwrap_or("N/A".into()),
        Err(_) => "N/A".into(),
    }  
}

fn time() -> String {
    match Command::new("date").arg("+%d-%m-%Y").output() {
        Ok(output) =>  String::from_utf8(output.stdout).unwrap_or("N/A".into()),
        Err(_) => "N/A".into(),
    }  
}

fn release() -> &'static str {
    let debug: bool = var("DEBUG").unwrap().parse().unwrap();

    match debug {
        true => "DEV", false => "RELEASE",
    }
}

fn dist_repo() -> String {
    match var("PACWRAP_DIST_REPO") {
        Ok(var) => var,
        Err(_) => "file:///usr/share/pacwrap/dist-repo".into(),
    }
}


fn main() {
    println!("cargo:rustc-env=PACWRAP_DIST_REPO={}", dist_repo());
    println!("cargo:rustc-env=PACWRAP_BUILDSTAMP={}", head());
    println!("cargo:rustc-env=PACWRAP_BUILDTIME={}", time());
    println!("cargo:rustc-env=PACWRAP_BUILD={}", release());
}
