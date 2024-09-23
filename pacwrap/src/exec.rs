/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
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
    fs::{remove_file, File},
    os::unix::io::AsRawFd,
    path::Path,
    process::{Child, Command},
    thread,
    time::Duration,
    vec::Vec,
};

use command_fds::{CommandFdExt, FdMapping};
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use signal_hook::iterator::Signals;

use pacwrap_core::{
    config::{
        self,
        register::{register_dbus, register_filesystems, register_permissions},
        ContainerHandle,
        ContainerType::Slice,
        Dbus,
    },
    constants::{
        BWRAP_EXECUTABLE,
        DBUS_PROXY_EXECUTABLE,
        DBUS_SOCKET,
        DEFAULT_PATH,
        IS_COLOR_TERMINAL,
        SIGNAL_LIST,
        XDG_RUNTIME_DIR,
    },
    err,
    error,
    exec::{
        args::{Argument, ExecutionArgs},
        fakeroot_container,
        path::check_path,
        seccomp::{configure_bpf_program, provide_bpf_program},
        utils::{decode_info_json, wait_on_container},
        ExecutionError,
        ExecutionType::Interactive,
    },
    impl_error,
    utils::{
        self,
        arguments::{Arguments, InvalidArgument, Operand as Op},
        check_root,
        env_var,
        TermControl,
    },
    Error,
    ErrorGeneric,
    ErrorKind,
    ErrorTrait,
    Result,
};

static SOCKET_SLEEP_DURATION: Duration = Duration::from_micros(500);

#[derive(Debug)]
enum ExecError {
    NestedNamespaceEnablement,
    ConsoleSessionRetention,
    SeccompDisablement,
}

impl_error!(ExecError);

impl Display for ExecError {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::SeccompDisablement => write!(fmter, "Disabling seccomp filtering can allow for sandbox escape."),
            Self::NestedNamespaceEnablement =>
                write!(fmter, "Namespace nesting has been known in the past to enable container escape vulnerabilities."),
            Self::ConsoleSessionRetention => write!(
                fmter,
                "Retaining a console session is known to allow for container escape. See CVE-2017-5226 for details."
            ),
        }
    }
}

enum ExecParams<'a> {
    FakeRoot(i8, bool, Vec<&'a str>, ContainerHandle<'a>),
    Container(i8, bool, Vec<&'a str>, ContainerHandle<'a>),
}

impl<'a> ExecParams<'a> {
    fn parse(args: &'a mut Arguments) -> Result<Self> {
        let mut verbosity: i8 = 0;
        let mut shell = matches!(args[0], Op::Value("shell"));
        let mut root = false;
        let mut container = None;
        let mut pos = 1;

        for str in args.inner() {
            if str.starts_with("-") || *str == "run" || *str == "shell" {
                pos += 1;
                continue;
            }

            break;
        }

        while let Some(arg) = args.next() {
            match arg {
                Op::Long("root") | Op::Short('r') => root = true,
                Op::Long("shell") | Op::Short('s') => shell = true,
                Op::Long("verbose") | Op::Short('v') => verbosity += 1,
                Op::LongPos(_, str) | Op::ShortPos(_, str) | Op::Value(str) =>
                    if container.is_none() {
                        container = Some(str);
                        break;
                    },
                _ => args.invalid_operand()?,
            }
        }

        let handle = match container {
            Some(container) => config::provide_handle(container)?,
            None => err!(InvalidArgument::TargetUnspecified)?,
        };
        let runtime = args.into_inner(pos);

        if let (Slice, false, ..) = (handle.metadata().container_type(), root, shell) {
            err!(ErrorKind::Message("Execution in container filesystem segments is not supported."))?
        }

        check_root()?;
        Ok(match root {
            true => Self::FakeRoot(verbosity, shell, runtime, handle),
            false => Self::Container(verbosity, shell, runtime, handle),
        })
    }
}

pub fn execute<'a>(args: &'a mut Arguments<'a>) -> Result<()> {
    match ExecParams::parse(args)? {
        ExecParams::FakeRoot(verbosity, true, _, handle) => execute_fakeroot(&handle, None, verbosity),
        ExecParams::FakeRoot(verbosity, false, args, handle) => execute_fakeroot(&handle, Some(args), verbosity),
        ExecParams::Container(verbosity, true, _, handle) => execute_container(&handle, vec!["bash"], true, verbosity),
        ExecParams::Container(verbosity, false, args, handle) => execute_container(&handle, args, false, verbosity),
    }
}

fn execute_container(ins: &ContainerHandle, arguments: Vec<&str>, shell: bool, verbosity: i8) -> Result<()> {
    let mut exec = ExecutionArgs::new();
    let mut jobs: Vec<Child> = Vec::new();
    let cfg = ins.config();
    let vars = ins.vars();
    let dbus = !cfg.dbus().is_empty();

    if !cfg.allow_forking() {
        exec.push_env(Argument::DieWithParent);
    }

    match !cfg.enable_userns() {
        true => exec.push_env(Argument::DisableNamespaces),
        false => error!(ExecError::NestedNamespaceEnablement).warn(),
    }

    match !cfg.retain_session() {
        true => exec.push_env(Argument::NewSession),
        false => error!(ExecError::ConsoleSessionRetention).warn(),
    }

    match shell && *IS_COLOR_TERMINAL {
        true => exec.env("TERM", "xterm"),
        false => exec.env("TERM", "dumb"),
    }

    if dbus {
        jobs.push(instantiate_dbus_proxy(cfg.dbus(), &mut exec, verbosity)?);
    }

    exec.env("XDG_RUNTIME_DIR", &XDG_RUNTIME_DIR);
    register_filesystems(cfg.filesystem(), vars, &mut exec)?;
    register_permissions(cfg.permissions(), &mut exec)?;

    let path = match exec.obtain_env("PATH") {
        Some(var) => var,
        None => {
            exec.env("PATH", DEFAULT_PATH);
            DEFAULT_PATH
        }
    };
    let path_vec: Vec<&str> = path.split(":").collect();
    let info_pipe = os_pipe::pipe().unwrap();
    let info_fd = info_pipe.1.as_raw_fd();
    let sec_pipe = os_pipe::pipe().unwrap();
    let sec_fd = match cfg.seccomp() {
        true => provide_bpf_program(configure_bpf_program(cfg), &sec_pipe.0, sec_pipe.1).unwrap(),
        false => {
            error!(ExecError::SeccompDisablement).warn();
            0
        }
    };
    let term_control = TermControl::new(0);
    let mut proc = Command::new(BWRAP_EXECUTABLE);
    let proc = if sec_fd == 0 {
        proc.env_clear()
            .args(exec.arguments())
            .arg("--info-fd")
            .arg(info_fd.to_string())
            .fd_mappings(vec![FdMapping {
                parent_fd: info_fd,
                child_fd: info_fd,
            }])
            .unwrap()
    } else {
        proc.env_clear()
            .args(exec.arguments())
            .arg("--info-fd")
            .arg(info_fd.to_string())
            .arg("--seccomp")
            .arg(sec_fd.to_string())
            .fd_mappings(vec![
                FdMapping {
                    parent_fd: info_fd,
                    child_fd: info_fd,
                },
                FdMapping {
                    parent_fd: sec_fd,
                    child_fd: sec_fd,
                },
            ])
            .unwrap()
    };

    match verbosity {
        0 => (),
        1 => eprintln!("Arguments:\t     {arguments:?}\n{ins:?}"),
        _ => eprintln!("Arguments:\t     {arguments:?}\n{ins:?}\n{exec:?}"),
    }

    check_path(ins, &arguments, path_vec)?;

    match proc.args(arguments).spawn() {
        Ok(child) => wait_on_container(
            child,
            term_control,
            decode_info_json(info_pipe)?,
            *cfg.allow_forking(),
            match !jobs.is_empty() {
                true => Some(jobs),
                false => None,
            },
            signal_trap,
            cleanup,
        ),
        Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())),
    }
}

fn execute_fakeroot(ins: &ContainerHandle, arguments: Option<Vec<&str>>, verbosity: i8) -> Result<()> {
    let arguments = match arguments {
        None => vec!["bash"],
        Some(args) => args,
    };

    if verbosity > 0 {
        eprintln!("Arguments:\t     {arguments:?}\n{ins:?}");
    }

    check_path(ins, &arguments, vec!["/usr/bin", "/bin"])?;
    fakeroot_container(Interactive, Some(signal_trap), ins, arguments)
}

fn signal_trap(bwrap_pid: i32) {
    let mut signals = Signals::new(*SIGNAL_LIST).unwrap();

    thread::Builder::new()
        .name("pacwrap-signal".to_string())
        .spawn(move || {
            let proc: &str = &format!("/proc/{}/", bwrap_pid);
            let proc = Path::new(proc);

            for _ in signals.forever() {
                if proc.exists() {
                    kill(Pid::from_raw(bwrap_pid), Signal::SIGKILL).unwrap();
                }
            }
        })
        .unwrap();
}

fn instantiate_dbus_proxy(per: &[Box<dyn Dbus>], args: &mut ExecutionArgs, verbosity: i8) -> Result<Child> {
    let dbus_socket_path = format!("/run/user/{}/bus", nix::unistd::geteuid());
    let dbus_session = env_var("DBUS_SESSION_BUS_ADDRESS")?;
    let mut dbus = Command::new(DBUS_PROXY_EXECUTABLE);

    register_dbus(per, args)?;
    create_placeholder(&DBUS_SOCKET)?;
    dbus.arg(dbus_session).arg(&*DBUS_SOCKET);

    if verbosity > 1 {
        dbus.arg("--log");
    }

    match dbus.arg("--filter").args(args.get_dbus()).spawn() {
        Ok(mut child) => {
            let mut increment: u8 = 0;

            args.robind(&DBUS_SOCKET, &dbus_socket_path);
            args.symlink(&dbus_socket_path, "/run/dbus/system_bus_socket");
            args.env("DBUS_SESSION_BUS_ADDRESS", &format!("unix:path={dbus_socket_path}"));

            /*
             * This blocking code is required to prevent a downstream race condition with
             * bubblewrap. Unless xdg-dbus-proxy is passed improper parameters, this while loop
             * shouldn't almost ever increment more than once or twice.
             *
             * With a sleep duration of 500 microseconds, we check the socket 200 times before failure.
             *
             * ADDENDUM: Upon further examination of bubblewrap's code, it is not possible to ask bubblewrap
             * to wait on a FD prior to instantiating the filesystem bindings.
             */

            while !check_socket(&DBUS_SOCKET, &increment, &mut child)? {
                increment += 1;
            }

            Ok(child)
        }
        Err(error) => err!(ErrorKind::ProcessInitFailure(DBUS_PROXY_EXECUTABLE, error.kind()))?,
    }
}

fn check_socket(socket: &String, increment: &u8, process_child: &mut Child) -> Result<bool> {
    if increment == &200 {
        process_child.kill().ok();
        remove_file(&*DBUS_SOCKET).prepend_io(|| DBUS_SOCKET.to_string())?;
        err!(ExecutionError::SocketTimeout(socket.into()))?
    }

    thread::sleep(SOCKET_SLEEP_DURATION);
    Ok(utils::check_socket(socket))
}

fn create_placeholder(path: &str) -> Result<()> {
    match File::create(path) {
        Ok(file) => {
            drop(file);
            Ok(())
        }
        Err(error) => err!(ErrorKind::IOError(path.into(), error.kind())),
    }
}

fn cleanup() -> Result<()> {
    if Path::new(&*DBUS_SOCKET).exists() {
        remove_file(&*DBUS_SOCKET).prepend_io(|| DBUS_SOCKET.to_string())?;
    }

    Ok(())
}
