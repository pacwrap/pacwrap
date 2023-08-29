use std::rc::Rc;
use std::{thread, time::Duration};
use std::process::{Command, Child, ExitStatus, exit};
use std::fs::{File, remove_file};
use std::io::Read;
use std::path::Path;
use std::vec::Vec;
use std::os::unix::io::AsRawFd;

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
    InstanceHandle};
use crate::utils::{TermControl, 
    Arguments, 
    env_var, 
    print_error, 
    print_warning};

pub mod args;
pub mod utils;

pub fn execute() {
    let mut root = false;
    let mut cmd = false;
    let mut shell = false;
    let mut verbose = false;

    let args = Arguments::new()
        .prefix("-E").ignore("--exec")
        .switch("-r", "--root", &mut root)
        .switch("-v", "--verbose", &mut verbose)
        .switch("-c", "--command", &mut cmd) 
        .switch("-s", "--shell", &mut shell)
        .parse_arguments()
        .require_target(1);

    let mut runtime = args.get_runtime().clone();
    let handle = &config::provide_handle(&runtime.remove(0));

    if verbose { 
        handle.vars().debug(handle.config(), &runtime); 
    }

    if root && cmd { 
        execute_fakeroot_container(handle, runtime); 
    } else if root && shell { 
        execute_fakeroot_container(handle, vec!("bash".into())); 
    } else if shell { 
        execute_container(handle, vec!("bash".into()), shell, verbose); 
    } else { 
        execute_container(handle, runtime, false, verbose); 
    } 
}

fn execute_container(ins: &InstanceHandle, arguments: Vec<Rc<str>>, shell: bool, verbose: bool) {
    let mut exec_args = ExecutionArgs::new();
    let mut jobs: Vec<Child> = Vec::new();
    let cfg = ins.config();
    let vars = ins.vars();

    if shell { exec_args.env("TERM", "xterm"); }    
    if ! cfg.allow_forking() { exec_args.push_env("--die-with-parent"); }
    if ! cfg.retain_session() { exec_args.push_env("--new-session"); } else {
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
    
    if verbose { println!("{:?} ",exec_args); }

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


fn signal_trap(pids: Vec<u64>, block: bool) {
    let mut signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM]).unwrap();

    thread::spawn(move || {
        for _sig in signals.forever() { 
            if block { 
                for pid in pids.iter() {
                    if Path::new(&format!("/proc/{}/", pid)).exists() { 
                        kill(Pid::from_raw(*pid as i32), Signal::SIGKILL).unwrap(); 
                    }
                }
            }
        }
    });
}

fn wait_on_process(mut process: Child, value: Value, block: bool, mut jobs: Vec<Child>, tc: TermControl) {  
    let bwrap_pid = value["child-pid"].as_u64().unwrap_or_default();
    let proc: &str = &format!("/proc/{}/", bwrap_pid);
    let mut j: Vec<u64> = [bwrap_pid].to_vec();
    
    for job in jobs.iter_mut() { 
        j.push(job.id() as u64); 
    }

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
        Ok(condition) => {
            match condition {
                Some(b) => {
                    p.register(args);
                    if let Condition::SuccessWarn(warning) = b {
                        print_warning(format!("{}: {} ", p.module(), warning));
                    }
                },
                None => continue
            }
        },
        Err(condition) => 
            match condition {
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
 
    match utils::fakeroot_container(ins, arguments.iter().map(|a| a.as_ref()).collect()) {
        Ok(process) => wait_on_process(process, json!(null), false, Vec::<Child>::new(), TermControl::new(0)),
        Err(_) => print_error("Failed to initialise bwrap."), 
    }
}

