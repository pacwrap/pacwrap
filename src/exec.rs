#![allow(unused_assignments)]

use std::env;
use std::process;
use std::process::{Command, Child, ExitStatus, exit};
use std::{thread, time::Duration};
use std::os::unix::io::AsRawFd;
use std::fs::{File, remove_file};
use std::io::{Read};
use std::path::Path;
use std::vec::Vec;
use std::collections::HashMap;

use signal_hook::{consts::SIGINT, iterator::Signals};
use os_pipe::{PipeReader, PipeWriter};
use std::cell::RefCell;
use command_fds::{CommandFdExt, FdMapping};
use serde_json::Value;

use crate::config::{self, Instance, vars::InsVars, filesystem::Filesystem, permission::Permission, dbus::Dbus};
use crate::utils::{print_error, print_warning, test_root, arguments::Arguments};
use crate::constants::BWRAP_EXECUTABLE;
use crate::exec::args::ExecutionArgs;

pub mod args;

pub fn execute() {
    let args = Arguments::new(1, "-E", HashMap::from([("--exec".into(), "E".into()),
                                                      ("--root".into(), "r".into()),
                                                      ("--shell".into(), "s".into()),
                                                      ("--command".into(),"c".into()),]));

    let switch = args.get_switch();
    let instance = args.get_targets()[0].clone();
    let runtime = args.get_runtime();

    let instance_vars = InsVars::new(&instance);
    let cfg = config::load_configuration(&instance_vars.config_path()); 

    if switch.contains("v") { instance_vars.debug(&cfg, &switch, &runtime); }

    match switch.as_str() {
        s if s.contains("rc") || s.contains("cr") => execute_fakeroot(instance_vars, runtime), 
        s if s.contains("rs") || s.contains("sr") => execute_fakeroot(instance_vars, &["bash".into()].to_vec()),
        s if s.contains("s") => execute_container(instance_vars,&["bash".into()].to_vec(), cfg, switch),
        &_ => execute_container(instance_vars, runtime, cfg, switch), 
    }
}

fn execute_container(vars: InsVars, arguments: &Vec<String>, cfg: Instance, switch: &String)  {
    let mut exec_args = ExecutionArgs::new(&["--tmpfs".into(), "/tmp".into()], 
                                           &["--dev".into(), "/dev".into(), "--proc".into(), "/proc".into()], &[]);
    let mut jobs: Vec<RefCell<Child>> = Vec::new();

    if ! cfg.allow_forking() { exec_args.push_env("--die-with-parent"); }
    if ! cfg.retain_session() { exec_args.push_env("--new-session"); }
    if switch.contains("s") { exec_args.env("TERM", "xterm"); }
    if cfg.dbus().len() > 0 { 
        jobs.push(register_dbus(cfg.dbus(), &vars, &mut exec_args).into()); } 

    register_filesystems(cfg.filesystem(), &vars, &mut exec_args);
    register_permissions(cfg.permissions(), &vars, &mut exec_args);
   
    //TODO: Implement separate abstraction for path vars.

    exec_args.env("PATH", "/usr/bin/:/bin");
    exec_args.env("XDG_RUNTIME_DIR", format!("/run/user/{}/", nix::unistd::geteuid()));
    
    if switch.contains("v") { println!("{:?} ",exec_args); }

    let (reader, writer) = os_pipe::pipe().unwrap();
    let fd = writer.as_raw_fd();
    let mut proc = Command::new(BWRAP_EXECUTABLE);
    
    proc.args(exec_args.get_bind()).args(exec_args.get_dev())
    .arg("--proc").arg("/proc").arg("--unshare-all").arg("--clearenv")
    .arg("--info-fd").arg(fd.to_string())
    .args(exec_args.get_env()).args(arguments)
    .fd_mappings(vec![FdMapping { parent_fd: fd, child_fd: fd }]).unwrap();  

    match proc.spawn() {
            Ok(c) => wait_on_process(c, &read_info_json(reader, writer), *cfg.allow_forking(), jobs),
            Err(_) => print_error(format!("Failed to initialise bwrap."))
    }
}

fn read_info_json(mut reader: PipeReader, writer: PipeWriter) -> Value { 
    let mut output = String::new();
    drop(writer);
    reader.read_to_string(&mut output).unwrap();           
    match serde_json::from_str(&output) {
        Ok(value) => value,
        Err(_) => Value::Null
    }
}

fn signal_init(child_id: String) {
    let mut signals = Signals::new(&[SIGINT]).unwrap();
    thread::spawn(move || {
        for _sig in signals.forever() {
            if Path::new(&format!("/proc/{}/", &child_id)).exists() { 
                Command::new("/usr/bin/kill").arg("-9").arg(&child_id).output().expect("Failed.");
            }
            clean_up_socket();
            Command::new("/usr/bin/reset").arg("-w").output().expect("Failed.");
            println!();
        }
    });
}

fn wait_on_process(mut process: Child, value: &Value, block: bool, jobs: Vec<RefCell<Child>>) {  
    if block {
        signal_init(value["child-pid"].to_string()); 
    }

    match process.wait() {
        Ok(status) => { 
            if block {
                let proc = format!("/proc/{}/", value["child-pid"]);

                while Path::new(&proc).exists() { 
                    thread::sleep(Duration::from_millis(250)); 
                }
            }

            if jobs.len() > 0 {
                for job in jobs.iter() {
                    let mut child = job.borrow_mut();
                    let _ = child.kill();
                }
            }

            clean_up_socket();
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
            println!();
            eprintln!("bwrap process {}", status);
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

fn register_permissions(per: &Vec<Box<dyn Permission>>, vars: &InsVars, args: &mut ExecutionArgs) {
    for p in per.iter() {
        match p.check() {
            Ok(_) => p.register(args, vars),
            Err(e) => print_warning(format!("Failed to register permission {}: {} ", e.module(), e.error()))
        }
    }
}

fn register_dbus(per: &Vec<Box<dyn Dbus>>, vars: &InsVars, args: &mut ExecutionArgs) -> Child {
    for p in per.iter() {
        p.register(args, vars);
    }

    let dbus_socket_path = format!("/run/user/{}/bus", nix::unistd::geteuid());
    let dbus_socket = create_dbus_socket();
    let dbus_session = env!("DBUS_SESSION_BUS_ADDRESS", "Failure");

    match Command::new("xdg-dbus-proxy")
    .arg(dbus_session).arg(&dbus_socket)
    .args(args.get_dbus()).spawn() {
         Ok(child) => {
            args.robind(dbus_socket, &dbus_socket_path);
            args.symlink(&dbus_socket_path, "/run/dbus/system_bus_socket");
            args.env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", &dbus_socket_path));
            child
        },
        Err(_) => {
            print_error("Activation of xdg-dbus-proxy failed.".into());
            exit(2); 
        },
    }
}


fn create_dbus_socket() -> String {
    let socket_address = format!("/run/user/1000/pacwrap_dbus_{}", &process::id());

    match File::create(&socket_address) {
        Ok(file) =>  {
            drop(file);
            socket_address 
        },
        Err(_) => {
            print_error(format!("Failed to create dbus socket."));
            eprintln!("Ensure you have write permissions to /run/user/.");
            String::new()
        }
    }
}

fn clean_up_socket() {
    let socket_address = format!("/run/user/1000/pacwrap_dbus_{}", &process::id());

    if ! Path::new(&socket_address).exists() {
        return;
    }

    if let Err(_) = remove_file(socket_address) {
        print_error(format!("Failed to remove FD."));
    }
}

fn execute_fakeroot(instance: InsVars, arguments: &Vec<String>) { 
    test_root(&instance);
    
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
    .arg("fakechroot")
    .arg("fakeroot")
    .args(arguments)
    .spawn() {
        Ok(process) => wait_on_process(process, &Value::Null, false, Vec::new()),
        Err(_) => print_error(format!("Failed to initialise bwrap.")), 
    }
}

