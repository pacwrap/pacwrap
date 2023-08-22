use std::{process::{Child, Command}, io::Error};

use serde_json::Value;

use crate::{constants::BWRAP_EXECUTABLE, 
    config::InstanceHandle, 
    utils::handle_process};

pub fn job_i32(process: &mut Child) -> i32 {
    match process.id().try_into() { Ok(i) => i, Err(_) => 0 }
}

pub fn derive_bwrap_child(value: &Value) -> i32 {
    match serde_json::from_value(value.clone()) { Ok(u) => u, Err(_) => 0 }
}

pub fn execute_in_container(ins: &InstanceHandle, arguments: Vec<&str>) {
    handle_process(fakeroot_container(ins, arguments))
}

pub fn fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<Child, Error> {  
    Command::new(BWRAP_EXECUTABLE)
    .env_clear()
    .arg("--tmpfs").arg("/tmp")
    .arg("--bind").arg(ins.vars().root()).arg("/")
    .arg("--ro-bind").arg("/usr/lib/libfakeroot").arg("/usr/lib/libfakeroot/")
    .arg("--ro-bind").arg("/usr/bin/fakeroot").arg("/usr/bin/fakeroot")
    .arg("--ro-bind").arg("/usr/bin/fakechroot").arg("/usr/bin/fakechroot")
    .arg("--ro-bind").arg("/usr/bin/faked").arg("/usr/bin/faked")
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
    .arg("--bind").arg(&ins.vars().pacman_sync).arg("/var/lib/pacman/sync")
    .arg("--bind").arg(&ins.vars().pacman_gnupg).arg("/etc/pacman.d/gnupg")
    .arg("--bind").arg(&ins.vars().pacman_cache).arg("/var/cache/pacman/pkg")
    .arg("--bind").arg(ins.vars().home()).arg(ins.vars().home_mount())  
    .arg("--dev").arg("/dev")
    .arg("--proc").arg("/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("FakeChroot")
    .arg("--new-session")
    .arg("--setenv").arg("TERM").arg("xterm")
    .arg("--setenv").arg("PATH").arg("/usr/bin")
    .arg("--setenv").arg("CWD").arg(ins.vars().home_mount())
    .arg("--setenv").arg("HOME").arg(ins.vars().home_mount())
    .arg("--setenv").arg("USER").arg(ins.vars().user())
    .arg("--die-with-parent")
    .arg("--unshare-user")
    .arg("--disable-userns")
    .arg("fakechroot")
    .arg("fakeroot")
    .args(arguments)
    .spawn()
}
