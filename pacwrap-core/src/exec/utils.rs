use std::{process::{Child, Command, Stdio}, io::Error, env::var};

use crate::{constants::{BWRAP_EXECUTABLE, self}, 
    config::InstanceHandle, 
    utils::handle_process, ErrorKind};

pub fn execute_agent(ins: &InstanceHandle) -> Result<Child,Error> { 
    let dist_img = option_env!("PACWRAP_DIST_IMG").unwrap_or("/usr/lib/pacwrap/runtime");
    let dist_tls = option_env!("PACWRAP_DIST_TLS").unwrap_or("/etc/ca-certificates/extracted/tls-ca-bundle.pem"); 

    Command::new(BWRAP_EXECUTABLE)
    .env_clear()
    .stdin(Stdio::piped())
    .arg("--bind").arg(&ins.vars().root()).arg("/mnt")
    .arg("--tmpfs").arg("/tmp")
    .arg("--tmpfs").arg("/etc")
    .arg("--ro-bind").arg(format!("{}/lib", dist_img)).arg("/lib64")
    .arg("--ro-bind").arg(format!("{}/bin", dist_img)).arg("/bin")
    .arg("--symlink").arg("/mnt/usr").arg("/usr")
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime") 
    .arg("--ro-bind").arg(dist_tls).arg("/etc/ssl/certs/ca-certificates.crt")
    .arg("--bind").arg(*constants::LOG_LOCATION).arg("/tmp/pacwrap.log") 
    .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/tmp/pacman/gnupg")
    .arg("--bind").arg(ins.vars().pacman_cache()).arg("/tmp/pacman/pkg")
    .arg("--ro-bind").arg(env!("PACWRAP_DIST_REPO")).arg("/tmp/dist-repo")
    .arg("--dev").arg("/dev")
    .arg("--dev").arg("/mnt/dev")
    .arg("--proc").arg("/mnt/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("pacwrap-agent")
    .arg("--new-session")
    .arg("--setenv").arg("PATH").arg("/bin")
    .arg("--setenv").arg("TERM").arg(env_var("TERM"))
    .arg("--setenv").arg("LD_LIBRARY_PATH").arg("/lib64:/usr/lib")
    .arg("--setenv").arg("LD_PRELOAD").arg("/lib64/libfakeroot.so:/lib64/libfakechroot.so")
    .arg("--setenv").arg("HOME").arg("/tmp")
    .arg("--setenv").arg("COLORTERM").arg(env_var("COLORTERM"))
    .arg("--setenv").arg("LANG").arg(env_var("LANG"))
    .arg("--setenv").arg("RUST_BACKTRACE").arg("1")
    .arg("--die-with-parent")
    .arg("--unshare-user")
    .arg("--disable-userns")
    .arg("agent")
    .arg("transact")
    .spawn()
}

pub fn execute_in_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<(), ErrorKind> {
    handle_process(&*BWRAP_EXECUTABLE, fakeroot_container(ins, arguments))
}

pub fn fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<Child, Error> {  
    Command::new(BWRAP_EXECUTABLE)
    .env_clear()
    .arg("--tmpfs").arg("/tmp")
    .arg("--bind").arg(ins.vars().root()).arg("/")
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
    .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/etc/pacman.d/gnupg")
    .arg("--bind").arg(ins.vars().pacman_cache()).arg("/var/cache/pacman/pkg")
    .arg("--bind").arg(ins.vars().home()).arg(ins.vars().home_mount())  
    .arg("--dev").arg("/dev")
    .arg("--proc").arg("/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("FakeChroot")
    .arg("--new-session")
    .arg("--setenv").arg("TERM").arg("xterm")
    .arg("--setenv").arg("PATH").arg("/usr/local/bin:/usr/bin")
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

fn env_var(env: &str) -> String {
    match var(env) {
        Ok(var) => var,
        Err(_) => String::new()
    }
}
