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

use crate::config::{self, Instance, InsVars, Filesystem, Permission, Dbus, permission::*};
use crate::utils::{self, TermControl, Arguments, env_var, print_error, print_warning};
use crate::constants::{BWRAP_EXECUTABLE, XDG_RUNTIME_DIR, DBUS_SOCKET};
use crate::exec::args::ExecutionArgs;

pub mod args;

pub fn execute() {
    let mut root = false;
    let mut cmd = false;
    let mut shell = false;
    let mut verbose = false;

    let args = Arguments::new()
        .prefix("-E")
        .switch("-r", "--root", &mut root)
        .switch("-v", "--verbose", &mut verbose)
        .switch("-c", "--command", &mut cmd) 
        .switch("-s", "--shell", &mut shell)
        .parse_arguments();

    let mut runtime = args.get_runtime().clone();

    args.require_target(1);

    let instance = runtime.remove(0);
    let instance_vars = InsVars::new(&instance);


    let cfg = config::load_configuration(&instance_vars.config_path()); 

    if verbose { instance_vars.debug(&cfg, &String::new(), &runtime); }

        if root && cmd { execute_fakeroot(instance_vars, &runtime) }
        else if root && shell { execute_fakeroot(instance_vars, &["bash".into()].to_vec()); }
        else if shell { execute_container(instance_vars,&["bash".into()].to_vec(), cfg, shell, verbose); }
        else { execute_container(instance_vars, &runtime, cfg, false, verbose); } 
}

fn execute_container(vars: InsVars, arguments: &Vec<String>, cfg: Instance, shell: bool, verbose: bool) {
    let mut exec_args = ExecutionArgs::new();
    let mut jobs: Vec<Child> = Vec::new();

    if shell { exec_args.env("TERM", "xterm"); }    
    if ! cfg.allow_forking() { exec_args.push_env("--die-with-parent"); }
    if ! cfg.retain_session() { exec_args.push_env("--new-session"); } else {
        print_warning(format!("Retaining a console session is known to allow for sandbox escape. See CVE-2017-5226 for details.")); 
    }

    if ! cfg.enable_userns() { 
        exec_args.push_env("--unshare-user"); 
        exec_args.push_env("--disable-userns"); 
    }

    if cfg.dbus().len() > 0 { jobs.push(register_dbus(cfg.dbus(), &vars, &mut exec_args).into()); } 

    register_filesystems(cfg.filesystem(), &vars, &mut exec_args);
    register_permissions(cfg.permissions(), &vars, &mut exec_args);
   
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
        .arg(fd.to_string()).args(arguments)
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
    let bwrap_pid = utils::derive_bwrap_child(&value["child-pid"]);
    let mut j: Vec<i32> = [bwrap_pid].to_vec();
    
    for job in jobs.iter_mut() { 
        j.push(utils::job_i32(job)); 
    }

    signal_trap(j, block.clone()); 

    match process.wait() {
        Ok(status) => { 
            if block {
                let proc = format!("/proc/{}/", bwrap_pid);

                while Path::new(&proc).exists() { 
                    thread::sleep(Duration::from_millis(250)); 
                }
            }

            for job in jobs.iter_mut() {
                job.kill().unwrap();
            } 
            
            clean_up_socket();
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
        None => { eprint!("\nbwrap process {}\n", status); exit(2); }
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

fn register_permissions(per: &Vec<Box<dyn Permission>>, vars: &InsVars, args: &mut ExecutionArgs) {
    for p in per.iter() {
        match p.check() {
        Ok(condition) => {
            match condition {
                Some(b) => {
                    p.register(args, vars);
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

fn register_dbus(per: &Vec<Box<dyn Dbus>>, vars: &InsVars, args: &mut ExecutionArgs) -> Child {
    for p in per.iter() {
        p.register(args, vars);
    }

    create_dbus_socket();

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

fn check_socket(socket: &String, increment: &u8, child: &mut Child) -> bool {
    if increment == &200 { 
        let _ = child.kill();

        print_error("xdg-dbux-proxy socket timed out.");
        clean_up_socket();
        exit(2); 
    }

    thread::sleep(Duration::from_micros(500));
    utils::check_socket(socket)
}

fn create_dbus_socket() {
    match File::create(&*DBUS_SOCKET) {
        Ok(file) =>  {
            drop(file);
        },
        Err(_) => {
            print_error("Failed to create dbus socket.");
            eprintln!("Ensure you have write permissions to /run/user/.");
            exit(2);
        }
    }
}

fn clean_up_socket() { 
    if ! Path::new(&*DBUS_SOCKET).exists() {
       return;
    }

    if let Err(_) = remove_file(&*DBUS_SOCKET) {
        print_error(format!("Failed to remove FD."));
    }
}

fn execute_fakeroot(instance: InsVars, arguments: &Vec<String>) { 
    let tc = TermControl::new(0);
    
    utils::test_root(&instance);
 
    match Command::new(BWRAP_EXECUTABLE)
    .arg("--tmpfs").arg("/tmp")
    .arg("--bind").arg(&instance.root()).arg("/")
    .arg("--ro-bind").arg("/usr/lib/libfakeroot").arg("/usr/lib/libfakeroot/")
    .arg("--ro-bind").arg("/usr/bin/fakeroot").arg("/usr/bin/fakeroot")
    .arg("--ro-bind").arg("/usr/bin/fakechroot").arg("/usr/bin/fakechroot")
    .arg("--ro-bind").arg("/usr/bin/faked").arg("/usr/bin/faked")
    .arg("--ro-bind").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
    .arg("--ro-bind").arg("/etc/localtime").arg("/etc/localtime")
    .arg("--bind").arg(&instance.pacman_sync).arg("/var/lib/pacman/sync")
    .arg("--bind").arg(&instance.pacman_gnupg).arg("/etc/pacman.d/gnupg")
    .arg("--bind").arg(&instance.pacman_cache).arg("/var/cache/pacman/pkg")
    .arg("--ro-bind").arg(&instance.pacman_mirrorlist).arg("/etc/pacman.d/mirrorlist")
    .arg("--ro-bind").arg(&instance.sync()).arg("/etc/pacman.conf")
    .arg("--ro-bind").arg(&instance.syncdb()).arg("/tmp/pacman.conf")
    .arg("--bind").arg(&instance.home()).arg(&instance.home_mount())  
    .arg("--dev").arg("/dev")
    .arg("--proc").arg("/proc")
    .arg("--unshare-all").arg("--share-net")
    .arg("--clearenv")
    .arg("--hostname").arg("FakeChroot")
    .arg("--new-session")
    .arg("--setenv").arg("TERM").arg("xterm")
    .arg("--setenv").arg("PATH").arg("/usr/bin")
    .arg("--setenv").arg("CWD").arg(&instance.home_mount())
    .arg("--setenv").arg("HOME").arg(&instance.home_mount())
    .arg("--setenv").arg("USER").arg(&instance.user())
    .arg("--die-with-parent")
    .arg("--unshare-user")
    .arg("--disable-userns")
    .arg("fakechroot")
    .arg("fakeroot")
    .args(arguments)
    .spawn() {
        Ok(process) => wait_on_process(process, json!(null), false, Vec::<Child>::new(), tc),
        Err(_) => print_error("Failed to initialise bwrap."), 
    }
}

