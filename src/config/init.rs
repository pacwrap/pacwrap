use std::{path::Path, process::exit, fs::File, io::Write};

use crate::{utils::print_error, constants::LOCATION};

static PACMAN_CONF_DEFAULT: &'static str = r###"
[options]
HoldPkg = pacman glibc
Architecture = auto 
LogFile = /tmp/pacman.log 
NoExtract = etc/pacman.conf etc/pacman.d/mirrorlist 

ParallelDownloads = 5 

SigLevel = Required DatabaseOptional
LocalFileSigLevel = Optional 

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist 

[multilib]
Include = /etc/pacman.d/mirrorlist
"###;

pub struct DirectoryLayout {
    dirs: Vec<&'static str>,
    root: &'static str,
}

impl DirectoryLayout {
    fn instantiate(self) {
        for dir in self.dirs {
            let path: &str = &(self.root.to_owned()+dir);
            let path = Path::new(path);
            
            if path.exists() {
                continue;
            }

            if let Err(err) = std::fs::create_dir_all(path) {
                print_error(format!("'{}' {err}", path.to_str().unwrap()));
                exit(1);
            }
        }
    }
}

fn data_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!("/root", "/home", "/state", "/pacman/gnupg", "/pacman/sync"),
        root: LOCATION.get_data(),
    }
}

fn cache_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!("/pkg"),
        root: LOCATION.get_cache()
    }
}

fn config_layout() -> DirectoryLayout {
    DirectoryLayout {
        dirs: vec!("/instance"),
        root: LOCATION.get_config()
    }
}

fn write_to_file(location: &str, contents: &str) {
    let location = Path::new(&location);

    if location.exists() {
        return;
    }

    let mut f = match File::create(&location) {
        Ok(f) => f,
        Err(error) => {
            print_error(format!("'{}': {}", location.to_str().unwrap(), error));
            exit(1); 
        }
    };
   
    if let Err(error) = write!(f, "{contents}") {
        print_error(format!("'{}': {}", location.to_str().unwrap(), error));
        exit(1);
    }
}

pub fn init() {
    config_layout().instantiate();
    data_layout().instantiate();
    cache_layout().instantiate();    
    write_to_file(&format!("{}/pacman.conf", LOCATION.get_config()), PACMAN_CONF_DEFAULT);
}
