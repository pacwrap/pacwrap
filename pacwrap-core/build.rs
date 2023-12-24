use std::env::var;

fn dist_repo() -> String {
    match var("PACWRAP_DIST_REPO") {
        Ok(var) => var,
        Err(_) => "file:///usr/share/pacwrap/dist-repo".into(),
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=PACWRAP_DIST_REPO");
    println!("cargo:rustc-env=PACWRAP_DIST_REPO={}", dist_repo());
}
