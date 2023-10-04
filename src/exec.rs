use std::rc::Rc;
use std::{thread, time::Duration};
use std::process::{Command, Child, ExitStatus, exit};
use std::fs::{File, remove_file};
use std::io::Read;
use std::path::Path;
use std::vec::Vec;
use std::os::unix::io::AsRawFd;

use console::style;
use nix::unistd::Pid;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;

use signal_hook::{consts::*, iterator::Signals};
use os_pipe::{PipeReader, PipeWriter};
use command_fds::{CommandFdExt, FdMapping};
use serde_json::{Value, json};

use crate::exec::args::ExecutionArgs;
use crate::constants::{BWRAP_EXECUTABLE, XDG_RUNTIME_DIR, DBUS_SOCKET};
use crate::config::{self, 
    InsVars, 
    Filesystem, 
    Permission, 
    Dbus, 
    permission::*, 
    InstanceHandle, InstanceType};
use crate::utils::{TermControl, 
    Arguments, 
    arguments,
    env_var, 
    print_error,
    print_help_error,
    print_warning};

pub mod args;
pub mod utils;

#[derive(Copy, Clone, Debug)]
enum Options {
    Shell,
    FakeRoot,
    Command,
    Container,
    Verbose,
    None
}

pub fn execute() {
    let mut mode = Options::Command;
    let mut root = Options::Container;
    let mut verbose = Options::None;
    let args = Arguments::new()
        .prefix("-E")
        .ignore("--exec")
        .short("-r").long("--root").map(&mut root).set(Options::FakeRoot).push()
        .short("-v").long("--verbose").map(&mut verbose).set(Options::Verbose).push()
        .short("-c").long("--command").map(&mut mode).set(Options::Command)
        .short("-s").long("--shell").set(Options::Shell).push()
        .assume_target()
        .parse_arguments()
        .require_target(1);
    let runtime = args.get_runtime().clone();
    let handle = &config::provide_handle(args.targets().get(0).as_ref().unwrap());

    if let InstanceType::DEP = handle.metadata().container_type() {
        print_error("Execution in dependencies is not supported.");
        exit(1);
    }

    if let Options::Verbose = verbose { 
        handle.vars().debug(handle.config(), &runtime); 
    }

    match root {
        Options::FakeRoot => 
            match mode {
                Options::Shell => execute_fakeroot_container(handle, vec!("bash".into())),
                Options::Command => execute_fakeroot_container(handle, runtime), 
                _ => arguments::invalid()
            }
        Options::Container => 
            match mode {
                Options::Shell => execute_container(handle, vec!("bash".into()), mode, verbose),
                Options::Command => execute_container(handle, runtime, mode, verbose),
                _=> arguments::invalid()
            }
        _ => unreachable!()
    }
}

fn execute_container(ins: &InstanceHandle, arguments: Vec<Rc<str>>, shell: Options, verbose: Options) {
    let mut exec_args = ExecutionArgs::new();
    let mut jobs: Vec<Child> = Vec::new();
    let cfg = ins.config();
    let vars = ins.vars();

    if let Options::Shell = shell { 
        exec_args.env("TERM", "xterm"); 
    }    
    
    if ! cfg.allow_forking() { 
        exec_args.push_env("--die-with-parent"); 
    }

    if ! cfg.retain_session() { 
        exec_args.push_env("--new-session"); 
    } else {
        print_warning("Retaining a console session is known to allow for sandbox escape. See CVE-2017-5226 for details."); 
    }

    if ! cfg.enable_userns() { 
        exec_args.push_env("--unshare-user"); 
        exec_args.push_env("--disable-userns"); 
    }

    if cfg.dbus().len() > 0 { 
        jobs.push(register_dbus(cfg.dbus(), &mut exec_args).into()); 
    } 

    register_filesystems(cfg.filesystem(), &vars, &mut exec_args);
    register_permissions(cfg.permissions(), &mut exec_args);
   
    //TODO: Implement separate abstraction for path vars.

    exec_args.env("PATH", "/usr/bin/:/bin");
    exec_args.env("XDG_RUNTIME_DIR", &*XDG_RUNTIME_DIR);

    if let Err(error) = check_path(ins, &arguments, vec!("/usr/bin", "/bin")) {
        print_help_error(error); 
    }

    if let Options::Verbose = verbose { 
        println!("{:?} ",exec_args); 
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
        .args(arguments.iter().map(|a| a.as_ref()).collect::<Vec<&str>>())
        .fd_mappings(vec![FdMapping { parent_fd: fd, child_fd: fd }]).unwrap();  

    match proc.spawn() {
            Ok(c) => wait_on_process(c, read_info_json(reader, writer), *cfg.allow_forking(), jobs, tc),
            Err(_) => print_error(format!("Failed to initialise bwrap."))
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
            Err(e) => {
                if *e.critical() {
                    print_error(format!("Failed to mount {}: {} ", e.module(), e.error()));
                    exit(1);
                } else {
                    print_warning(format!("Failed to mount {}: {} ", e.module(), e.error()));
                }
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
            args.env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", &dbus_socket_path));
            
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
    crate::utils::check_socket(socket)
}

fn create_socket(path: &str) {
    match File::create(path) {
        Ok(file) => drop(file),
        Err(error) => {
            print_error(format!("Failed to create socket '{}': {}", path, error));
            eprintln!("Ensure you have write permissions to /run/user/.");
            exit(2);
        }
    }
}

fn clean_up_socket(path: &str) { 
    if ! Path::new(path).exists() {
       return;
    }

    if let Err(error) = remove_file(path) {
        print_error(format!("'Failed to remove socket '{}': {}", path, error));
    }
}

fn execute_fakeroot_container(ins: &InstanceHandle, arguments: Vec<Rc<str>>) {  
    crate::utils::test_root(ins.vars());

    if let Err(error) = check_path(ins, &arguments, vec!("/usr/bin", "/bin")) {
        print_help_error(error); 
    }

    match utils::fakeroot_container(ins, arguments.iter().map(|a| a.as_ref()).collect()) {
        Ok(process) => wait_on_process(process, json!(null), false, Vec::<Child>::new(), TermControl::new(0)),
        Err(_) => print_error("Failed to initialise bwrap."), 
    }
}

fn check_path(ins: &InstanceHandle, args: &Vec<Rc<str>>, path: Vec<&str>) -> Result<(), String> {
    if args.len() == 0 {
        Err(format!("Runtime parameters not specified."))?
    }

    let exec = args.get(0).unwrap().as_ref();
    let root = ins.vars().root().as_ref();

    for dir in path {
        match Path::new(&format!("{}/{}",root,dir)).try_exists() {
            Ok(_) => { 
                let exists = dest_exists(root, dir, exec, 0)?; 

                if exists {
                    return Ok(());
                }
            },
            Err(error) => Err(&format!("Invalid {} variable '{}': {}", style("PATH").bold(), dir, error))?
        }
    }

    Err(format!("'{}' not available container {}.", exec, style("PATH").bold()))
}

fn dest_exists(root: &str, dir: &str, exec: &str, mut recursion: u8) -> Result<bool,String> {
    let path = format!("{}{}/{}", root, dir, exec);
    let path = Path::new(&path);
    let path_direct = format!("{}/{}", root, exec);
    let path_direct = Path::new(&path_direct);

    if recursion == 40 {
        Err(format!("'{}': Symbolic link recursion depth maximum of {} exceeded.", exec, style(recursion).bold()))?
    }

    recursion += 1;

    if path.is_symlink() {
        if let Ok(path) = path.read_link() {
            if let Some(path) = path.as_os_str().to_str() {
                return dest_exists(root, dir, path, recursion);
            }
        }
    } else if path.exists() {
        return Ok(true) 
    } else if path_direct.is_symlink() {
        if let Ok(path) = path_direct.read_link() {
            if let Some(path) = path.as_os_str().to_str() {
                return dest_exists(root, dir, path, recursion);
            } 
        }
    } else if path_direct.exists() {
        return Ok(true)
    }

    Ok(false)
}
