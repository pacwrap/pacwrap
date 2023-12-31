use std::os::unix::process::ExitStatusExt;
use std::{thread, time::Duration};
use std::process::{Command, Child, ExitStatus, exit};
use std::fs::{File, remove_file};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use std::os::unix::io::AsRawFd;

use nix::unistd::Pid;
use nix::sys::signal::{Signal, kill};

use signal_hook::{consts::*, iterator::Signals};
use os_pipe::{PipeReader, PipeWriter};
use command_fds::{CommandFdExt, FdMapping};
use serde_yaml::Value;

use pacwrap_core::{err,
    ErrorKind,
    error::*,
    exec::{ExecutionError,
        args::ExecutionArgs, 
        fakeroot_container,
    },
    constants::{self,
        DEFAULT_PATH,
        BWRAP_EXECUTABLE, 
        DBUS_PROXY_EXECUTABLE,
        DBUS_SOCKET},
    config::{self, 
        Dbus,        
        InstanceHandle, 
        InstanceType,
        register::{register_filesystems, 
            register_permissions, 
            register_dbus}},
    utils::{TermControl, 
        arguments::{Arguments, Operand, InvalidArgument},
        env_var, 
        print_warning}};

const PROCESS_SLEEP_DURATION: Duration = Duration::from_millis(250);
const SOCKET_SLEEP_DURATION: Duration = Duration::from_micros(500);

enum ExecParams<'a> {
    Root(i8, bool, Vec<&'a str>,  InstanceHandle<'a>),
    Container(i8, bool, Vec<&'a str>, InstanceHandle<'a>),
}

/*
 * Open an issue or possibly submit a PR if this module's argument parser 
 * is conflicting with an application you use.
 */

impl <'a>ExecParams<'a> {
    fn parse(args: &'a mut Arguments) -> Result<Self> {
        let runtime = args.values()
            .iter()
            .filter_map(|f| {
                let str = *f;

                match str {
                    string if string.starts_with("-E") 
                        | string.eq("--exec") 
                        | string.eq("--target")  
                        | string.eq("--verbose")
                        | string.eq("--shell") 
                        | string.eq("--root") 
                        | string.eq("--fake-chroot") => None,
                    _ => Some(str),
                }
            })
            .skip(1)
            .collect(); 
        let mut verbosity: i8 = 0;
        let mut shell = false;
        let mut root = false;
        let mut container = None;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('r') | Operand::Long("root") => root = true,
                Operand::Short('s') | Operand::Long("shell") => shell = true,
                Operand::Short('v') | Operand::Long("verbose") => verbosity += 1,
                Operand::ShortPos('E', str) 
                    | Operand::ShortPos('s', str)
                    | Operand::ShortPos('r', str)
                    | Operand::ShortPos('v', str)
                    | Operand::LongPos("target", str)
                    | Operand::Value(str) => if let None = container { 
                        container = Some(str); 
                    },
                _ => continue,
            }
        }

        let handle = match container {
            Some(container) => config::provide_handle(container)?,
            None => err!(InvalidArgument::TargetUnspecified)?,
        };

        if let InstanceType::DEP = handle.metadata().container_type() {
            err!(ErrorKind::Message("Execution upon sliced filesystems is not supported."))?
        }

        Ok(match root {
            true => Self::Root(verbosity, shell, runtime, handle),
            false => Self::Container(verbosity, shell, runtime, handle),
        })
    }
}

pub fn execute<'a>(args: &'a mut Arguments<'a>) -> Result<()> {
    match ExecParams::parse(args)? {
        ExecParams::Root(verbosity, true, _, handle) => execute_fakeroot_container(&handle, vec!("bash"), verbosity),
        ExecParams::Root(verbosity,  false, args, handle) => execute_fakeroot_container(&handle, args, verbosity),
        ExecParams::Container(verbosity, true, _, handle) => execute_container(&handle, vec!("bash"), true, verbosity),
        ExecParams::Container(verbosity, false, args, handle) => execute_container(&handle, args, false, verbosity),
    }
}

fn execute_container<'a>(ins: &InstanceHandle, arguments: Vec<&str>, shell: bool, verbosity: i8) -> Result<()> {
    let mut exec_args = ExecutionArgs::new();
    let mut jobs: Vec<Child> = Vec::new();
    let cfg = ins.config();
    let vars = ins.vars();
 
    if ! cfg.allow_forking() { 
        exec_args.push_env("--die-with-parent"); 
    }

    if ! cfg.enable_userns() { 
        exec_args.push_env("--unshare-user"); 
        exec_args.push_env("--disable-userns"); 
    }

    match ! cfg.retain_session() { 
        true => exec_args.push_env("--new-session"),
        false => print_warning("Retaining a console session is known to allow for sandbox escape. See CVE-2017-5226 for details."),
    }

    match shell && *constants::IS_COLOR_TERMINLAL { 
        true => exec_args.env("TERM", "xterm"),
        false => exec_args.env("TERM", "dumb"),
    } 

    if cfg.dbus().len() > 0 { 
        jobs.push(instantiate_dbus_proxy(cfg.dbus(), &mut exec_args)?); 
    } 

    register_filesystems(cfg.filesystem(), &vars, &mut exec_args)?;
    register_permissions(cfg.permissions(), &mut exec_args)?;
   
    let path = match exec_args.get_var("PATH") {
        Some(var) => var.as_str(), None => { exec_args.env("PATH", DEFAULT_PATH); DEFAULT_PATH },
    };
    let path_vec: Vec<&str> = path.split(":").collect();

    if verbosity == 1 {
        println!("Arguments:\t     {arguments:?}\n{ins:?}");
    } else if verbosity > 1 {
        println!("Arguments:\t     {arguments:?}\n{ins:?}\n{exec_args:?}");
    }

    check_path(ins, &arguments, path_vec)?;

    let (reader, writer) = os_pipe::pipe().unwrap();
    let fd = writer.as_raw_fd();
    let tc = TermControl::new(0);
    let mut proc = Command::new(BWRAP_EXECUTABLE);
      
    proc.arg("--dir").arg("/tmp")
        .args(exec_args.get_bind())
        .arg("--dev").arg("/dev")
        .arg("--proc").arg("/proc")
        .args(exec_args.get_dev())
        .arg("--unshare-all")
        .arg("--clearenv")
        .args(exec_args.get_env()).arg("--info-fd")
        .arg(fd.to_string())
        .args(arguments)
        .fd_mappings(vec![FdMapping { parent_fd: fd, child_fd: fd }]).unwrap();  

    match proc.spawn() {
        Ok(c) => Ok(wait_on_process(c, bwrap_json(reader, writer)?, *cfg.allow_forking(), jobs, tc))?,
        Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())), 
    }
}

fn signal_trap(bwrap_pid: i32) {
    let mut signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM]).unwrap();

    thread::spawn(move || {
        let proc: &str = &format!("/proc/{}/", bwrap_pid); 
        let proc = Path::new(proc);     

        for _ in signals.forever() { 
            if proc.exists() {
                kill(Pid::from_raw(bwrap_pid), Signal::SIGKILL).unwrap();
            }
        }
    });
}

fn wait_on_process(mut process: Child, bwrap_pid: i32, block: bool, mut jobs: Vec<Child>, tc: TermControl) -> Result<()> {  
    signal_trap(bwrap_pid);
 
    match process.wait() {
        Ok(status) => {
            if block {
                let proc: &str = &format!("/proc/{}/", bwrap_pid); 
                let proc = Path::new(proc); 

                while proc.exists() {
                    thread::sleep(PROCESS_SLEEP_DURATION);
                }
            }

            for job in jobs.iter_mut() {
                job.kill().unwrap();
            }
     
            if let Err(err) = cleanup(&tc) {
                err.warn();
            }

            process_exit(status)
        },
        Err(error) => err!(ErrorKind::ProcessWaitFailure(BWRAP_EXECUTABLE, error.kind()))
    }
}

fn cleanup(tc: &TermControl) -> Result<()> {
    if let Err(errno) = tc.reset_terminal() {
        print_warning(format!("Failed to restore termios parameters: {errno}"));
    }

    clean_up_socket(&*DBUS_SOCKET)?;
    Ok(())
}

fn process_exit(status: ExitStatus) -> Result<()> {
    exit(match status.code() {
        Some(status) => status,
        None => { 
            eprint!("\nbwrap process {status}"); 
            println!();  
            100+status.signal().unwrap_or(0) 
        }
    });
}

fn instantiate_dbus_proxy(per: &Vec<Box<dyn Dbus>>, args: &mut ExecutionArgs) -> Result<Child> {
    register_dbus(per, args)?;
    create_placeholder(&*DBUS_SOCKET)?;

    let dbus_socket_path = format!("/run/user/{}/bus", nix::unistd::geteuid());
    let dbus_session = env_var("DBUS_SESSION_BUS_ADDRESS")?;

    match Command::new(DBUS_PROXY_EXECUTABLE)
    .arg(dbus_session).arg(&*DBUS_SOCKET)
    .args(args.get_dbus()).spawn() {
        Ok(mut child) => {
            let mut increment: u8 = 0;
            
            args.robind(&*DBUS_SOCKET, &dbus_socket_path);
            args.symlink(&dbus_socket_path, "/run/dbus/system_bus_socket");
            args.env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={dbus_socket_path}"));
            
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

            while ! check_socket(&*DBUS_SOCKET, &increment, &mut child)? {
                increment += 1;
            }

            Ok(child)
        },
        Err(error) => err!(ErrorKind::ProcessInitFailure(DBUS_PROXY_EXECUTABLE, error.kind()))?
    }
}

fn check_socket(socket: &String, increment: &u8, process_child: &mut Child) -> Result<bool> {
    if increment == &200 { 
        process_child.kill().ok();
        clean_up_socket(&*DBUS_SOCKET)?;
        err!(ExecutionError::SocketTimeout(socket.into()))?
    }

    thread::sleep(SOCKET_SLEEP_DURATION);
    Ok(pacwrap_core::utils::check_socket(socket))
}

fn create_placeholder(path: &str) -> Result<()> {
    match File::create(path) {
        Ok(file) => Ok(drop(file)),
        Err(error) => err!(ErrorKind::IOError(path.into(), error.kind()))
    }
}

fn clean_up_socket(path: &str) -> Result<()> { 
    if let Err(error) = remove_file(path) {
        match error.kind() {
            std::io::ErrorKind::NotFound => return Ok(()),
            _ => err!(ErrorKind::IOError(path.into(), error.kind()))?,
        }
    }

    Ok(())
}

fn execute_fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>, verbosity: i8) -> Result<()> {  
    if verbosity > 0 {
        println!("Arguments:\t     {arguments:?}\n{ins:?}\n");
    }

    check_path(ins, &arguments, vec!("/usr/bin", "/bin"))?;

    match fakeroot_container(ins, arguments.iter().map(|a| a.as_ref()).collect()) {
        Ok(process) => Ok(wait_on_process(process, 0, false, Vec::<Child>::new(), TermControl::new(0)))?,
        Err(err) => err!(ErrorKind::ProcessInitFailure(BWRAP_EXECUTABLE, err.kind())), 
    }
}

fn bwrap_json(mut reader: PipeReader, writer: PipeWriter) -> Result<i32> { 
    let mut output = String::new();
  
    drop(writer);
    reader.read_to_string(&mut output).unwrap();    
   
    match serde_yaml::from_str::<Value>(&output) {
        Ok(value) => match value["child-pid"].as_u64() {
            Some(value) => Ok(value as i32), 
            None => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
        },
        Err(_) => err!(ErrorKind::Message("Unable to acquire child pid from bwrap process.")),
    }
}

fn check_path(ins: &InstanceHandle, args: &Vec<&str>, path: Vec<&str>) -> Result<()> {
    if args.len() == 0 {
        err!(ExecutionError::RuntimeArguments)?
    }

    let exec = *args.get(0).unwrap();
    let root = ins.vars().root().as_ref();

    for dir in path {
        match Path::new(&format!("{}/{}",root,dir)).try_exists() {
            Ok(_) => if dest_exists(root, dir, exec)? { return Ok(()) },
            Err(error) => err!(ExecutionError::InvalidPathVar(dir.into(), error.kind()))?
        }
    }

    err!(ExecutionError::ExecutableUnavailable(exec.into()))?
}

fn dest_exists(root: &str, dir: &str, exec: &str) -> Result<bool> {
    if exec.contains("..") {
        err!(ExecutionError::UnabsoluteExec(exec.into()))?
    } else if dir.contains("..") {
        err!(ExecutionError::UnabsolutePath(exec.into()))?
    }

    let path = format!("{}{}/{}", root, dir, exec);
    let path = obtain_path(Path::new(&path), exec)?;
    let path_direct = format!("{}/{}", root, exec);
    let path_direct = obtain_path(Path::new(&path_direct), exec)?;

    if path.is_dir() | path_direct.is_dir() {
        err!(ExecutionError::DirectoryNotExecutable(exec.into()))?
    } else if let Ok(path) = path.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return dest_exists(root, dir, path);
        }
    } else if let Ok(path) = path_direct.read_link() {
        if let Some(path) = path.as_os_str().to_str() {
            return dest_exists(root, dir, path);
        } 
    } 

    Ok(path.exists() | path_direct.exists())
}

fn obtain_path(path: &Path, exec: &str) -> Result<PathBuf> {
    match Path::canonicalize(&path) {
        Ok(path) => Ok(path), 
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
            _ => err!(ErrorKind::IOError(exec.into(), err.kind()))?,
        }
    }
}
