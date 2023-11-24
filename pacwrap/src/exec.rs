use std::{thread, time::Duration};
use std::process::{Command, Child, ExitStatus, exit};
use std::fs::{File, remove_file};
use std::io::{Read, ErrorKind};
use std::path::{Path, PathBuf};
use std::vec::Vec;
use std::os::unix::io::AsRawFd;

use nix::unistd::Pid;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;

use signal_hook::{consts::*, iterator::Signals};
use os_pipe::{PipeReader, PipeWriter};
use command_fds::{CommandFdExt, FdMapping};
use serde_json::{Value, json};

use pacwrap_core::{exec::args::ExecutionArgs, 
    exec::utils::fakeroot_container,
    constants::{self,
        BWRAP_EXECUTABLE, 
        XDG_RUNTIME_DIR, 
        DBUS_SOCKET, 
        RESET, 
        BOLD},
    config::{self, 
        InsVars,
        Filesystem, 
        Permission, 
        Dbus, 
        permission::*, 
        InstanceHandle, 
        InstanceType},
    utils::{TermControl, 
        arguments::{Arguments, Operand},
        env_var, 
        print_error,
        print_help_error,
        print_warning}};

enum ExecParams<'a> {
    Root(bool, bool, Vec<&'a str>,  InstanceHandle<'a>),
    Container(bool, bool, Vec<&'a str>, InstanceHandle<'a>),
}

/*
 * Open an issue or possibly submit a PR if this module's argument parser 
 * is conflicting with an application you use.
 */

impl <'a>From<&'a mut Arguments<'a>> for ExecParams<'a> {
    fn from(args: &'a mut Arguments) -> Self {
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
        let mut verbose = false;
        let mut shell = false;
        let mut root = false;
        let mut container = None;

        while let Some(arg) = args.next() {
            match arg {
                Operand::Short('r') | Operand::Long("root") => root = true,
                Operand::Short('s') | Operand::Long("shell") => shell = true,
                Operand::Short('v') | Operand::Long("verbose") => verbose = true,
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
            Some(container) => match config::provide_handle(container) {
                Ok(container) => container,
                Err(error) => { 
                    print_error(error);
                    exit(1);
                }
            },
            None => {
                print_help_error("Target not specified.");
                exit(1);
            },
        };

        if let InstanceType::DEP = handle.metadata().container_type() {
            print_error("Execution in dependencies is not supported.");
            exit(1);
        }

        match root {
            true => Self::Root(verbose, shell, runtime, handle),
            false => Self::Container(verbose, shell, runtime, handle),
        }
    }
}

pub fn execute<'a>(args: &'a mut Arguments<'a>) {
    match args.into() {
        ExecParams::Root(verbose, true, _, handle) => execute_fakeroot_container(&handle, vec!("bash"), verbose),
        ExecParams::Root(verbose,  false, args, handle) => execute_fakeroot_container(&handle, args, verbose),
        ExecParams::Container(verbose, true, _, handle) => execute_container(&handle, vec!("bash"), true, verbose),
        ExecParams::Container(verbose, false, args, handle) => execute_container(&handle, args, false, verbose),
    }
}

fn execute_container(ins: &InstanceHandle, arguments: Vec<&str>, shell: bool, verbose: bool) {
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
        jobs.push(register_dbus(cfg.dbus(), &mut exec_args).into()); 
    } 

    register_filesystems(cfg.filesystem(), &vars, &mut exec_args);
    register_permissions(cfg.permissions(), &mut exec_args);
   
    //TODO: Implement separate abstraction for path vars.

    exec_args.env("PATH", "/usr/local/bin:/usr/bin/:/bin:");
    exec_args.env("XDG_RUNTIME_DIR", &*XDG_RUNTIME_DIR);

    if verbose { 
        ins.vars().debug(ins.config(), &arguments); 
        println!("{:?} ",exec_args); 
    }

    if let Err(error) = check_path(ins, &arguments, vec!("/usr/local/bin/", "/usr/bin", "/bin")) {
        print_help_error(error); 
    }

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
        Ok(c) => wait_on_process(c, read_info_json(reader, writer), *cfg.allow_forking(), jobs, tc),
        Err(err) => print_error(format!("Failed to initialize '{BWRAP_EXECUTABLE}': {err}"))
    }
}

fn read_info_json(mut reader: PipeReader, writer: PipeWriter) -> Value { 
    let mut output = String::new();
    drop(writer);
    reader.read_to_string(&mut output).unwrap();           
    match serde_json::from_str(&output) {
        Ok(value) => value,
        Err(_) => json!(null)
    }
}

fn signal_trap(pids: Vec<i32>, block: bool) {
    let mut signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM]).unwrap();

    thread::spawn(move || {
        for _sig in signals.forever() { 
            if block { 
                for pid in pids.iter() {
                    if Path::new(&format!("/proc/{}/", pid)).exists() { 
                        kill(Pid::from_raw(*pid), Signal::SIGKILL).unwrap(); 
                    }
                }
            }
        }
    });
}

fn wait_on_process(mut process: Child, value: Value, block: bool, mut jobs: Vec<Child>, tc: TermControl) {  
    let bwrap_pid = value["child-pid"].as_u64().unwrap_or_default();
    let proc: &str = &format!("/proc/{}/", bwrap_pid);
    let mut j: Vec<i32> = jobs.iter_mut()
        .map(|j| j.id() as i32)
        .collect::<Vec<_>>();

    j.push(bwrap_pid as i32);
    signal_trap(j, block.clone()); 

    match process.wait() {
        Ok(status) => { 
            if block {                
                while Path::new(proc).exists() { 
                    thread::sleep(Duration::from_millis(250)); 
                }
            }

            for job in jobs.iter_mut() {
                job.kill().unwrap();
            } 
            
            clean_up_socket(&*DBUS_SOCKET);
            tc.reset_terminal().unwrap();
            process_exit(status);
        },
        Err(_) => {
            print_error(format!("bwrap process abnormally terminated."));
            exit(2);
        }
    }
}

fn process_exit(status: ExitStatus) {
    match status.code() {
        Some(o) => exit(o),
        None => { 
            eprint!("\nbwrap process {}\n", status); 
            exit(2); 
        }
    }
}

fn register_filesystems(per: &Vec<Box<dyn Filesystem>>, vars: &InsVars, args: &mut ExecutionArgs) {
    for p in per.iter() {
       match p.check(vars) {
            Ok(_) => p.register(args, vars),
            Err(e) => if *e.critical() {
                print_error(format!("Failed to mount {}: {} ", e.module(), e.error()));
                exit(1);
            } else {
                print_warning(format!("Failed to mount {}: {} ", e.module(), e.error()));
            }
        }
    }
}

fn register_permissions(per: &Vec<Box<dyn Permission>>, args: &mut ExecutionArgs) {
    for p in per.iter() {
        match p.check() {
            Ok(condition) => match condition {
                Some(b) => {
                    p.register(args);
                    
                    if let Condition::SuccessWarn(warning) = b {
                        print_warning(format!("{}: {} ", p.module(), warning));
                    }
                },
                None => continue
            },
            Err(condition) => match condition {
                PermError::Warn(error) => {
                    print_warning(format!("Failed to register permission {}: {} ", p.module(), error));
                },
                PermError::Fail(error) => {
                    print_error(format!("Failed to register permission {}: {} ", p.module(), error));
                    exit(1);
                }
            }
        }    
    }
}

fn register_dbus(per: &Vec<Box<dyn Dbus>>, args: &mut ExecutionArgs) -> Child {
    for p in per.iter() {
        p.register(args);
    }

    create_socket(&*DBUS_SOCKET);

    let dbus_socket_path = format!("/run/user/{}/bus", nix::unistd::geteuid());
    let dbus_session = env_var("DBUS_SESSION_BUS_ADDRESS");

    match Command::new("xdg-dbus-proxy")
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

            while ! check_socket(&*DBUS_SOCKET, &increment, &mut child) {
                increment += 1;
            }

            child
        },
        Err(_) => {
            print_error("Activation of xdg-dbus-proxy failed.");
            exit(2); 
        },
    }
}

fn check_socket(socket: &String, increment: &u8, process_child: &mut Child) -> bool {
    if increment == &200 { 
        process_child.kill().ok();
        print_error(format!("Socket '{}': timed out.", socket));
        clean_up_socket(&*DBUS_SOCKET);
        exit(2); 
    }

    thread::sleep(Duration::from_micros(500));
    pacwrap_core::utils::check_socket(socket)
}

fn create_socket(path: &str) {
    match File::create(path) {
        Ok(file) => drop(file),
        Err(error) => {
            print_error(format!("Failed to create socket '{path}': {error}"));
            exit(2);
        }
    }
}

fn clean_up_socket(path: &str) { 
    if let Err(error) = remove_file(path) {
        match error.kind() {
            ErrorKind::NotFound => return,
            _ => print_error(format!("'Failed to remove socket '{path}': {error}")),
        }
    }
}

fn execute_fakeroot_container(ins: &InstanceHandle, arguments: Vec<&str>, verbose: bool) {  
    if verbose { 
        ins.vars().debug(ins.config(), &arguments); 
    }

    if let Err(error) = check_path(ins, &arguments, vec!("/usr/bin", "/bin")) {
        print_help_error(error); 
    }

    match fakeroot_container(ins, arguments.iter().map(|a| a.as_ref()).collect()) {
        Ok(process) => wait_on_process(process, json!(null), false, Vec::<Child>::new(), TermControl::new(0)),
        Err(err) => print_error(format!("Failed to initialize '{BWRAP_EXECUTABLE}:' {err}")), 
    }
}

fn check_path(ins: &InstanceHandle, args: &Vec<&str>, path: Vec<&str>) -> Result<(), String> {
    if args.len() == 0 {
        Err(format!("Runtime arguments not specified."))?
    }

    let exec = args.get(0).unwrap();
    let root = ins.vars().root().as_ref();

    for dir in path {
        match Path::new(&format!("{}/{}",root,dir)).try_exists() {
            Ok(_) => if dest_exists(root, dir, exec)? { return Ok(()) },
            Err(error) => Err(&format!("Invalid {}PATH{} variable '{dir}': {error}", *BOLD, *RESET))?
        }
    }

    Err(format!("'{exec}': Not available in container {}PATH{}.", *BOLD, *RESET))
}

fn dest_exists(root: &str, dir: &str, exec: &str) -> Result<bool,String> {
    if exec.contains("..") {
        Err(format!("'{exec}': Executable path must be absolute."))?
    } else if dir.contains("..") {
        Err(format!("'{dir}': {}PATH{} variable must be absolute.", *BOLD, *RESET))?
    }

    let path = format!("{}{}/{}", root, dir, exec);
    let path = obtain_path(Path::new(&path), exec)?;
    let path_direct = format!("{}/{}", root, exec);
    let path_direct = obtain_path(Path::new(&path_direct), exec)?;

    if path.is_dir() | path_direct.is_dir() {
        Err(format!("'{exec}': Directories are not executable."))?
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

fn obtain_path(path: &Path, exec: &str) -> Result<PathBuf,String> {
    match Path::canonicalize(&path) {
        Ok(path) => Ok(path), 
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Ok(path.to_path_buf()),
            _ => Err(format!("'{exec}': {err}"))? 
        }
    }
}
