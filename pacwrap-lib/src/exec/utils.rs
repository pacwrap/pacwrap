use std::{process::{Child, Command, Stdio}, io::Error};

use crate::{constants::BWRAP_EXECUTABLE, 
    config::InstanceHandle, 
    utils::{handle_process, env_var}};

pub fn execute_pacwrap_dist(ins: &InstanceHandle) -> Result<Child,Error> {  
    Command::new(BWRAP_EXECUTABLE)
    .env_clear()
    .stdin(Stdio::piped())
    .arg("--bind").arg(ins.vars().root().as_ref()).arg("/")
    .arg("--tmpfs").arg("/tmp")
    .arg("--ro-bind").arg(option_env!("PACWRAP_DIST_LIBS").unwrap_or("/usr/lib/pacwrap/lib/")).arg("/tmp/lib/") 
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
    .arg("--bind").arg(&ins.vars().pacman_gnupg.as_ref()).arg("/tmp/pacman/gnupg")
    .arg("--bind").arg(&ins.vars().pacman_cache.as_ref()).arg("/tmp/pacman/pkg")
    .arg("--ro-bind").arg(env!("PACWRAP_DIST_REPO").split_at(7).1).arg("/tmp/dist-repo")
    .arg("--ro-bind").arg(option_env!("PACWRAP_DIST_EXEC").unwrap_or("/usr/lib/pacwrap/agent")).arg("/tmp/bin/agent")
    .arg("--dev").arg("/dev")
    .arg("--proc").arg("/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("FakeChroot")
    .arg("--new-session")
    .arg("--setenv").arg("TERM").arg(env_var("TERM"))
    .arg("--setenv").arg("LD_LIBRARY_PATH").arg("/tmp/lib")
    .arg("--setenv").arg("HOME").arg("/tmp")
    .arg("--setenv").arg("COLORTERM").arg(env_var("COLORTERM"))
    .arg("--setenv").arg("RUST_BACKTRACE").arg("1")
    .arg("--die-with-parent")
    .arg("--unshare-user")
    .arg("--disable-userns")
    .arg("/tmp/lib/ld-linux-x86-64.so.2")
    .arg("/tmp/bin/agent")
    .arg("transact")
    .spawn()
}

pub fn execute_in_container(ins: &InstanceHandle, arguments: Vec<&str>) {
    handle_process(fakeroot_container(ins, arguments))
}

pub fn fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<Child, Error> {  
    Command::new(BWRAP_EXECUTABLE)
    .env_clear()
    .arg("--tmpfs").arg("/tmp")
    .arg("--bind").arg(ins.vars().root().as_ref()).arg("/")
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
    .arg("--bind").arg(&ins.vars().pacman_gnupg.as_ref()).arg("/etc/pacman.d/gnupg")
    .arg("--bind").arg(&ins.vars().pacman_cache.as_ref()).arg("/var/cache/pacman/pkg")
    .arg("--bind").arg(ins.vars().home().as_ref()).arg(ins.vars().home_mount().as_ref())  
    .arg("--dev").arg("/dev")
    .arg("--proc").arg("/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("FakeChroot")
    .arg("--new-session")
    .arg("--setenv").arg("TERM").arg("xterm")
    .arg("--setenv").arg("PATH").arg("/usr/local/bin:/usr/bin")
    .arg("--setenv").arg("CWD").arg(ins.vars().home_mount().as_ref())
    .arg("--setenv").arg("HOME").arg(ins.vars().home_mount().as_ref())
    .arg("--setenv").arg("USER").arg(ins.vars().user().as_ref())
    .arg("--die-with-parent")
    .arg("--unshare-user")
    .arg("--disable-userns")
    .arg("fakechroot")
    .arg("fakeroot")
    .args(arguments)
    .spawn()
}
