/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier R.M. <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    fmt::{Display, Formatter},
    process::{Child, Command, Stdio},
};

use command_fds::{CommandFdExt, FdMapping};
use lazy_static::lazy_static;

use crate::{
    config::InstanceHandle,
    constants::{
        BOLD,
        BWRAP_EXECUTABLE,
        COLORTERM,
        GID,
        LANG,
        LOG_LOCATION,
        PACMAN_KEY_SCRIPT,
        RESET,
        RUNTIME_DIRECTORY,
        RUNTIME_TLS_STORE,
        TERM,
        UID,
    },
    err,
    exec::{
        seccomp::{provide_bpf_program, FilterType::*},
        utils::{agent_params, wait_on_process},
    },
    impl_error,
    sync::transaction::{TransactionMetadata, TransactionParameters},
    to_static_str,
    Error,
    ErrorKind,
    ErrorTrait,
    Result,
};

pub mod args;
pub mod seccomp;
pub mod utils;

lazy_static! {
    static ref ID: (&'static str, &'static str) = (to_static_str!(UID), to_static_str!(GID));
    static ref DIST_IMG: &'static str = option_env!("PACWRAP_DIST_IMG").unwrap_or(RUNTIME_DIRECTORY);
    static ref DIST_TLS: &'static str = option_env!("PACWRAP_DIST_TLS").unwrap_or(RUNTIME_TLS_STORE);
}

#[derive(Debug, Clone)]
pub enum ExecutionError {
    InvalidPathVar(String, std::io::ErrorKind),
    ExecutableUnavailable(String),
    RuntimeArguments,
    UnabsolutePath(String),
    UnabsoluteExec(String),
    DirectoryNotExecutable(String),
    SocketTimeout(String),
}

impl_error!(ExecutionError);

impl Display for ExecutionError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::InvalidPathVar(dir, err) => write!(fmter, "Invalid {}PATH{} variable '{dir}': {err}", *BOLD, *RESET),
            Self::ExecutableUnavailable(exec) => write!(fmter, "'{}': Not available in container {}PATH{}.", exec, *BOLD, *RESET),
            Self::UnabsolutePath(path) => write!(fmter, "'{}': {}PATH{} variable must be absolute", path, *BOLD, *RESET),
            Self::UnabsoluteExec(path) => write!(fmter, "'{}': Executable path must be absolute.", path),
            Self::DirectoryNotExecutable(path) => write!(fmter, "'{}': Directories are not executables.", path),
            Self::SocketTimeout(socket) => write!(fmter, "Socket '{socket}': timed out."),
            Self::RuntimeArguments => write!(fmter, "Invalid runtime arguments."),
        }
    }
}

#[rustfmt::skip]
pub fn fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>) -> Result<Child> {
	let (sec_reader, sec_writer) = os_pipe::pipe().unwrap();
	let sec_fd = provide_bpf_program(vec![Standard, Namespaces], &sec_reader, sec_writer).unwrap();
	let fd_mappings = vec![FdMapping { 
	        parent_fd: sec_fd, 
	        child_fd: sec_fd }];
	
	match Command::new(BWRAP_EXECUTABLE).env_clear()
	    .arg("--tmpfs").arg("/tmp")
		.arg("--bind").arg(ins.vars().root()).arg("/")
        .arg("--ro-bind").arg(format!("{}/lib", *DIST_IMG)).arg("/tmp/runtime")
        .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
        .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
        .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/etc/pacman.d/gnupg")
        .arg("--bind").arg(ins.vars().pacman_cache()).arg("/var/cache/pacman/pkg")
        .arg("--bind").arg(ins.vars().home()).arg(ins.vars().home_mount())  
        .arg("--dev").arg("/dev")
        .arg("--proc").arg("/proc")
        .arg("--unshare-all").arg("--share-net")
        .arg("--hostname").arg("FakeChroot")
        .arg("--new-session")
        .arg("--setenv").arg("TERM").arg("xterm")
        .arg("--setenv").arg("PATH").arg("/usr/local/bin:/usr/bin")
        .arg("--setenv").arg("CWD").arg(ins.vars().home_mount())
        .arg("--setenv").arg("HOME").arg(ins.vars().home_mount())
        .arg("--setenv").arg("USER").arg(ins.vars().user())
        .arg("--die-with-parent")
        .arg("--disable-userns")
        .arg("--unshare-user")
        .arg("--seccomp")
        .arg(sec_fd.to_string())
        .arg("fakechroot")
        .arg("fakeroot")
        .args(arguments)
        .fd_mappings(fd_mappings)
        .unwrap()
        .spawn() 
	{
		Ok(child) => Ok(child),
		Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())),
	}
}

#[rustfmt::skip]
pub fn transaction_agent(ins: &InstanceHandle, params: &TransactionParameters, metadata: &TransactionMetadata) -> Result<Child> { 
	let (sec_reader, sec_writer) = os_pipe::pipe().unwrap();
	let (params_reader, params_writer) = os_pipe::pipe().unwrap();  	
	let params_fd = agent_params(&params_reader, &params_writer, params, metadata)?;
	let sec_fd = provide_bpf_program(vec![Standard, Namespaces], &sec_reader, sec_writer).unwrap();
	let fd_mappings = vec![
		FdMapping { 
		    parent_fd: sec_fd, 
		    child_fd: sec_fd 
		}, 
		FdMapping { 
		    parent_fd: params_fd, 
		    child_fd: params_fd 
		},
	];
	
	match Command::new(BWRAP_EXECUTABLE).env_clear()
        .arg("--bind").arg(&ins.vars().root()).arg("/mnt/fs")
        .arg("--symlink").arg("/mnt/fs/usr").arg("/usr")
        .arg("--ro-bind").arg(format!("{}/lib", *DIST_IMG)).arg("/lib64")
        .arg("--ro-bind").arg(format!("{}/bin", *DIST_IMG)).arg("/bin")
        .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
        .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime") 
        .arg("--ro-bind").arg(*DIST_TLS).arg("/etc/ssl/certs/ca-certificates.crt")
        .arg("--bind").arg(*LOG_LOCATION).arg("/mnt/share/pacwrap.log") 
        .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/mnt/share/gnupg")
        .arg("--bind").arg(ins.vars().pacman_cache()).arg("/mnt/share/cache")
        .arg("--ro-bind").arg(env!("PACWRAP_DIST_REPO")).arg("/mnt/share/dist-repo")
        .arg("--dev").arg("/dev")
        .arg("--dev").arg("/mnt/fs/dev")
        .arg("--proc").arg("/mnt/fs/proc")
        .arg("--unshare-all")
        .arg("--share-net")
        .arg("--hostname").arg("pacwrap-agent")
        .arg("--new-session")
        .arg("--setenv").arg("HOME").arg("/tmp")
        .arg("--setenv").arg("PATH").arg("/bin")
        .arg("--setenv").arg("TERM").arg(*TERM)	
        .arg("--setenv").arg("LANG").arg(*LANG)
        .arg("--setenv").arg("COLORTERM").arg(*COLORTERM) 
        .arg("--setenv").arg("LD_LIBRARY_PATH").arg("/lib64:/usr/lib")
        .arg("--setenv").arg("LD_PRELOAD").arg("/lib64/libfakeroot.so:/lib64/libfakechroot.so")
        .arg("--setenv").arg("PACWRAP_REAL_UID").arg(ID.0)
        .arg("--setenv").arg("PACWRAP_REAL_GID").arg(ID.1)
        .arg("--setenv").arg("RUST_BACKTRACE").arg("1")
        .arg("--die-with-parent")
        .arg("--unshare-user")
        .arg("--disable-userns")
        .arg("--seccomp")
        .arg(sec_fd.to_string())
        .arg("--ro-bind-data")
        .arg(params_fd.to_string())
        .arg("/mnt/agent_params")
        .arg("agent")
        .arg("transact")
        .fd_mappings(fd_mappings)
        .unwrap()
        .spawn() 
	{
		Ok(child) => Ok(child),
		Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())),
    }
}

pub fn pacman_key(path: &str, cmd: Vec<&str>) -> Result<()> {
    match Command::new(PACMAN_KEY_SCRIPT)
        .stderr(Stdio::null())
        .env("EUID", "0")
        .arg("--gpgdir")
        .arg(path)
        .args(cmd)
        .spawn()
    {
        Ok(proc) => wait_on_process(PACMAN_KEY_SCRIPT, proc),
        Err(error) => err!(ErrorKind::ProcessInitFailure(PACMAN_KEY_SCRIPT, error.kind()))?,
    }
}
