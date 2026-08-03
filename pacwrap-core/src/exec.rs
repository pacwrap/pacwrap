/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2026 Xavier Moffett <sapphirus@azorium.net>
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
    io::ErrorKind as IOErrorKind,
    os::{fd::AsRawFd, unix::process::ExitStatusExt},
    process::{Child, Command, ExitStatus, Stdio},
};

use command_fds::{CommandFdExt, FdMapping};
use thiserror::Error as ThisError;

use crate::{
    Error,
    ErrorTrait,
    ErrorType,
    PathContext,
    Result,
    config::{ContainerHandle, ContainerType},
    constants::{
        BOLD,
        BWRAP_EXECUTABLE,
        COLORTERM,
        DEFAULT_PATH,
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
    exec::{
        seccomp::{FilterType::*, provide_bpf_program},
        utils::{agent_params, decode_info_json, wait_on_fakeroot},
    },
    lazy_lock,
    sync::transaction::{TransactionFlags, TransactionMetadata, TransactionParameters},
    to_static_str,
    utils::TermControl,
};

pub mod args;
pub mod path;
pub mod seccomp;
pub mod utils;

lazy_lock! {
    static ref ID: (&'static str, &'static str) = (to_static_str!(UID), to_static_str!(GID));
    static ref DIST_IMG: &'static str = option_env!("PACWRAP_DIST_IMG").unwrap_or(RUNTIME_DIRECTORY);
    static ref DIST_TLS: &'static str = option_env!("PACWRAP_DIST_TLS").unwrap_or(RUNTIME_TLS_STORE);
}

#[derive(ThisError, Debug, Clone)]
pub enum ExecutionError {
    #[error("Invalid {bold}PATH{reset} variable '{0}': {1}", bold=*BOLD, reset=*RESET)]
    InvalidPathVar(String, IOErrorKind),
    #[error("'{0}': Not available in container {bold}PATH{reset}.", bold=*BOLD, reset=*RESET)]
    ExecutableUnavailable(String),
    #[error("Invalid runtime arguments.")]
    RuntimeArguments,
    #[error("'{0}': {bold}PATH{reset} variable must be absolute.", bold=*BOLD, reset=*RESET)]
    UnabsolutePath(String),
    #[error("'{0}': Executable path must be absolute.")]
    UnabsoluteExec(String),
    #[error("'{0}': Directories are not executables.")]
    DirectoryNotExecutable(String),
    #[error("Socket '{0}': timed out.")]
    SocketTimeout(String),
    #[error("Container exited with code: {0}")]
    Container(i32),
    #[error("bubblewrap exited with {0}")]
    Bwrap(ExitStatus),
}

impl ErrorTrait for ExecutionError {
    fn code(&self) -> i32 {
        match self {
            Self::Container(status) => *status,
            Self::Bwrap(status) => 128 + status.signal().unwrap_or(0),
            _ => 1,
        }
    }
}

impl From<ExecutionError> for Error<ErrorType> {
    fn from(value: ExecutionError) -> Self {
        Self {
            code: value.code(),
            error: value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionType {
    Interactive,
    NonInteractive,
}

pub fn fakeroot_container(
    exec_type: ExecutionType,
    trap: Option<fn(i32)>,
    ins: &ContainerHandle,
    arguments: Vec<&str>,
) -> Result<()> {
    let term_control = TermControl::new(0);
    let info_pipe = os_pipe::pipe().expect("bwrap pipe");
    let sec_pipe = os_pipe::pipe().expect("eBPF pipe");
    let sec_fd = provide_bpf_program(vec![Standard, Namespaces], &sec_pipe.0, sec_pipe.1).expect("eBPF program");
    let info_fd = info_pipe.1.as_raw_fd();
    let fd_mappings = vec![
        FdMapping {
            parent_fd: sec_fd,
            child_fd: sec_fd,
        },
        FdMapping {
            parent_fd: info_fd,
            child_fd: info_fd,
        },
    ];
    let mut process = Command::new(BWRAP_EXECUTABLE);

    #[rustfmt::skip]
	process.env_clear()
        .arg("--tmpfs").arg("/tmp")
        .arg("--proc").arg("/proc")
        .arg("--dev").arg("/dev")
        .arg("--ro-bind").arg(*DIST_IMG).arg("/usr")
        .arg("--symlink").arg("usr/lib").arg("/lib")
        .arg("--symlink").arg("usr/lib").arg("/lib64")
        .arg("--symlink").arg("usr/bin").arg("/bin")
        .arg("--symlink").arg("usr/bin").arg("/sbin")
        .arg("--bind").arg(ins.vars().root()).arg("/mnt/fs")
        .arg("--dev").arg("/mnt/fs/dev")
        .arg("--unshare-all")
        .arg("--share-net")
        .arg("--new-session")
        .arg("--setenv").arg("LD_PRELOAD").arg("/lib64/libfakechroot.so")
        .arg("--setenv").arg("LD_LIBRARY_PATH").arg("/lib64:/usr/lib:/tmp/lib")
        .arg("--setenv").arg("TERM").arg("xterm")
        .arg("--setenv").arg("COLORTERM").arg(*COLORTERM)
        .arg("--setenv").arg("PATH").arg(DEFAULT_PATH)
        .arg("--setenv").arg("USER").arg(ins.vars().user())
        .arg("--setenv").arg("HOME").arg("/root")
        .arg("--die-with-parent")
        .arg("--disable-userns")
        .arg("--unshare-user")
        .arg("--seccomp")
        .arg(sec_fd.to_string())
        .arg("--info-fd")
        .arg(info_fd.to_string());

    #[rustfmt::skip]
    if let ContainerType::Slice = ins.metadata().container_type() {
        process.arg("--dir").arg("/root")  
            .arg("--ro-bind").arg(format!("{}/bin", *DIST_IMG)).arg("/mnt/fs/bin")
            .arg("--ro-bind").arg(format!("{}/lib", *DIST_IMG)).arg("/mnt/fs/lib64")
            .arg("--dir").arg("/mnt/fs/root") ;

        if arguments[0] == "ash" {
            process.arg("--hostname").arg("BusyBox").arg("--setenv").arg("ENV").arg("/etc/profile")
        } else {
            process.arg("--hostname").arg("FakeChroot").arg("fakeroot").arg("chroot").arg("/mnt/fs")
        }
    } else {
        process.arg("--hostname").arg("FakeChroot")
            .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
            .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/mnt/fs/etc/pacman.d/gnupg")
            .arg("--bind").arg(ins.vars().pacman_cache()).arg("/mnt/fs/var/cache/pacman/pkg")
            .arg("--bind").arg(ins.vars().home()).arg("/mnt/fs/root")
            .arg("--setenv").arg("EUID").arg("0") 
            .arg("--setenv").arg("PATH").arg(DEFAULT_PATH)
            .arg("fakeroot").arg("chroot").arg("/mnt/fs")
    };

    wait_on_fakeroot(
        exec_type,
        process
            .args(arguments)
            .fd_mappings(fd_mappings)
            .expect("FD Mappings")
            .spawn()
            .context_path(BWRAP_EXECUTABLE)?,
        term_control,
        decode_info_json(info_pipe)?,
        trap,
    )
}

pub fn transaction_agent(
    ins: &ContainerHandle,
    flags: &TransactionFlags,
    params: TransactionParameters,
    metadata: &TransactionMetadata,
) -> Result<Child> {
    let params_pipe = os_pipe::pipe().expect("params pipe");
    let params_fd = agent_params(&params_pipe.0, &params_pipe.1, &params, metadata)?;
    let sec_pipe = os_pipe::pipe().expect("eBPF pipe");
    let sec_fd = provide_bpf_program(vec![Standard, Namespaces], &sec_pipe.0, sec_pipe.1).expect("eBPF program");
    let fd_mappings = vec![
        FdMapping {
            parent_fd: sec_fd,
            child_fd: sec_fd,
        },
        FdMapping {
            parent_fd: params_fd,
            child_fd: params_fd,
        },
    ];
    let mut process = Command::new(BWRAP_EXECUTABLE);

    #[rustfmt::skip]
    process.arg("--bind").arg(ins.vars().root()).arg("/mnt/fs")
        .arg("--symlink").arg("/mnt/fs/usr").arg("/usr")
        .arg("--ro-bind").arg(format!("{}/bin", *DIST_IMG)).arg("/bin")
        .arg("--ro-bind").arg(format!("{}/lib", *DIST_IMG)).arg("/lib64")
        .arg("--symlink").arg("lib").arg("/lib")
        .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
        .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
        .arg("--ro-bind").arg(*DIST_TLS).arg("/etc/ssl/certs/ca-certificates.crt")
        .arg("--bind").arg(*LOG_LOCATION).arg("/mnt/share/pacwrap.log") 
        .arg("--bind").arg(ins.vars().pacman_gnupg()).arg("/mnt/share/gnupg")
        .arg("--bind").arg(ins.vars().pacman_cache()).arg("/mnt/share/cache")
        .arg("--dev").arg("/dev")
        .arg("--dev").arg("/mnt/fs/dev")
        .arg("--proc").arg("/mnt/fs/proc")
        .arg("--proc").arg("/proc")
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
        .arg("--setenv").arg("LD_PRELOAD").arg("/lib64/libfakechroot.so")
        .arg("--setenv").arg("PACWRAP_REAL_UID").arg(ID.0)
        .arg("--setenv").arg("PACWRAP_REAL_GID").arg(ID.1)
        .arg("--die-with-parent")
        .arg("--unshare-user")
        .arg("--disable-userns")
        .arg("--seccomp")
        .arg(sec_fd.to_string())
        .arg("--ro-bind-data")
        .arg(params_fd.to_string())
        .arg("/mnt/agent_params");

    if flags.contains(TransactionFlags::DEBUG) {
        process.arg("--setenv").arg("RUST_BACKTRACE").arg("full");
    }

    Ok(process
        .arg("agent")
        .arg("transact")
        .fd_mappings(fd_mappings)
        .expect("FD Mappings")
        .spawn()
        .context_path(BWRAP_EXECUTABLE)?)
}

pub fn pacwrap_key(cmd: Vec<&str>) -> Result<()> {
    Command::new(PACMAN_KEY_SCRIPT)
        .stderr(Stdio::null())
        .env("COLOURTERM", *COLORTERM)
        .args(cmd)
        .spawn()?
        .wait()
        .context_path(PACMAN_KEY_SCRIPT)?;

    Ok(())
}
