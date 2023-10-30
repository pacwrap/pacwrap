use std::process::Command;
use std::env::var;

fn head() -> String {
    match Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
        Ok(output) =>  String::from_utf8(output.stdout).unwrap_or("N/A".into()),
        Err(_) => "N/A".into(),
    }  
}

fn time(debug: bool) -> String {
    match debug {
        false => match Command::new("git").args(["log", "-1", "--date=format:%d/%m/%Y", "--format=%ad"]).output() {
            Ok(output) =>  String::from_utf8(output.stdout).unwrap_or("N/A".into()),
            Err(_) => "N/A".into(),
        },
        true => match Command::new("date").args(["+%d/%m/%Y %T"]).output() {
            Ok(output) =>  String::from_utf8(output.stdout).unwrap_or("N/A".into()),
            Err(_) => "N/A".into(),
        }
    }
}

fn release(debug: bool) -> &'static str {
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

fn is_debug() -> bool {
    var("DEBUG").unwrap().parse().unwrap()
}

fn main() {
    let debug: bool = is_debug();

    println!("cargo:rustc-env=PACWRAP_DIST_REPO={}", dist_repo());
    println!("cargo:rustc-env=PACWRAP_BUILDSTAMP={}", head());
    println!("cargo:rustc-env=PACWRAP_BUILDTIME={}", time(debug));
    println!("cargo:rustc-env=PACWRAP_BUILD={}", release(debug));
}
