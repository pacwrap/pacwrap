/*
 * pacwrap-core
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
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
    fmt::Write,
    fs::{read_dir, File},
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    result::Result as StdResult,
};

use crate::{config::ContainerCache, constants::CONTAINER_DIR, utils::print_warning, Error, ErrorGeneric};
use indexmap::IndexMap;

pub struct ProcessList {
    list: IndexMap<i32, Process>,
    groups: IndexMap<String, Vec<i32>>,
}

#[derive(Clone)]
pub struct Process {
    pid: i32,
    cmd: Vec<String>,
    stat: ProcStat,
    instance: String,
    depth: u32,
    fork: bool,
}

#[derive(Clone)]
pub struct ProcStat {
    parent: i32,
    thread_name: String,
}

impl ProcessList {
    fn new(map: IndexMap<i32, Process>, instances: IndexMap<String, Vec<i32>>) -> Self {
        Self {
            list: map,
            groups: instances,
        }
    }

    pub fn list(&self) -> Vec<&Process> {
        self.list.iter().map(|a| a.1).collect()
    }

    pub fn filter_by_target(&self, targets: &Vec<&str>) -> Vec<&Process> {
        self.list.iter().filter(|a| targets.contains(&a.1.instance())).map(|a| a.1).collect()
    }

    pub fn filter_by_pid(&self, targets: &Vec<i32>) -> Vec<&Process> {
        self.list.iter().filter(|a| targets.contains(&a.1.pid())).map(|a| a.1).collect()
    }

    pub fn keys_by_instance(&self, ins: &str) -> Option<&Vec<i32>> {
        self.groups.get(ins)
    }
}

impl Process {
    fn new(id: i32, level: u32, cmdline: Vec<String>, procstat: ProcStat, ins: String, forked: bool) -> Self {
        Self {
            pid: id,
            depth: level,
            cmd: cmdline,
            stat: procstat,
            instance: ins,
            fork: forked,
        }
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn exec_path(&self) -> &str {
        &self.cmd[0]
    }

    pub fn exec(&self) -> &str {
        match self.cmd[0].char_indices().filter(|c| c.1 == '/').last() {
            Some((index, ..)) => &self.cmd[0].split_at(index + 1).1,
            None => &self.cmd[0],
        }
    }

    pub fn cmdlist(&self) -> Vec<&str> {
        self.cmd.iter().map(|a| a.as_str()).collect()
    }

    pub fn cmdlist_string(&self, start: usize) -> Result<String, Error> {
        let mut string = String::new();

        for idx in start .. self.cmd.len() {
            write!(string, "{} ", self.cmd[idx]).prepend(|| format!("Failure writing string to cmd buffer."))?;
        }

        Ok(string)
    }

    pub fn stat(&self) -> &ProcStat {
        &self.stat
    }

    pub fn fork(&self) -> bool {
        self.fork
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }
}

impl ProcStat {
    fn new(pid: i32) -> Option<Self> {
        let stat = match File::open(&format!("/proc/{}/stat", pid)) {
            Ok(file) => file,
            Err(_) => return None,
        };
        let mut stat = BufReader::new(stat);
        let mut stat_str = String::new();
        let stat: Vec<&str> = match stat.read_to_string(&mut stat_str) {
            Ok(_) => stat_str.split(" ").collect(),
            Err(_) => return None,
        };

        Some(Self {
            thread_name: stat[1].into(),
            parent: stat[3].parse().unwrap_or(1),
        })
    }

    pub fn thread_name(&self) -> &str {
        &self.thread_name
    }

    pub fn parent(&self) -> i32 {
        self.parent
    }
}

pub fn list<'a>(cache: &'a ContainerCache<'a>) -> Result<ProcessList, Error> {
    let mut map: IndexMap<i32, Process> = IndexMap::new();
    let mut groups: IndexMap<String, Vec<i32>> = IndexMap::new();

    for pid in procfs()? {
        let cmdlist = match cmdlist(pid) {
            Some(cmdlist) => cmdlist,
            None => continue,
        };
        let stat = match ProcStat::new(pid) {
            Some(stat) => stat,
            None => continue,
        };

        if cmdlist[0] != "bwrap" && cmdlist[0] != "pacwrap" && !map.contains_key(&stat.parent()) {
            continue;
        }

        let check = qualify_process(&cmdlist, stat.parent(), &map);
        let (ins, depth, fork) = match check {
            Some(instance) => instance,
            None => continue,
        };

        match groups.get_mut(&ins) {
            Some(vec) => vec.push(pid),
            None => {
                if let None = cache.get_instance_option(&ins) {
                    print_warning(&format!("Container {ins} doesn't exist."));
                }

                groups.insert(ins.clone(), vec![pid]);
            }
        }

        map.insert(pid, Process::new(pid, depth, cmdlist, stat, ins, fork));
    }

    Ok(ProcessList::new(map, groups))
}

fn procfs() -> Result<Vec<i32>, Error> {
    Ok(read_dir("/proc/")
        .prepend_io(|| "/proc/".into())?
        .filter_map(StdResult::ok)
        .filter(|s| s.metadata().is_ok_and(|s| s.is_dir()))
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .expect("Invalid UTF-8 filename in procfs")
                .parse()
                .map_or_else(|_| None, |v| Some(v))
        })
        .collect())
}

fn cmdlist(pid: i32) -> Option<Vec<String>> {
    let list = match File::open(&format!("/proc/{}/cmdline", pid)) {
        Ok(file) => file,
        Err(_) => return None,
    };
    let mut list = BufReader::new(list);
    let mut cmdlist: Vec<String> = Vec::new();
    let mut data = Vec::new();
    let mut index = 0;

    while let Ok(len) = list.read_until(b'\0', &mut data) {
        if len == 0 {
            break;
        }

        data.remove(len - 1);
        cmdlist.push(String::from_utf8(data).unwrap_or_default());
        index += len;

        match list.seek(SeekFrom::Start(index as u64)) {
            Ok(_) => {
                data = Vec::new();
                continue;
            }
            Err(_) => break,
        }
    }

    if index == 0 {
        return None;
    } else if cmdlist.len() == 1 && cmdlist[0].contains(' ') {
        /*
         * For some strange reason, the Linux kernel will sometimes provide a non-nul delineated string;
         * therefore split it into an array ourselves when this does occur.
         *
         * Application this was observed happening with was chromium-based electron.
         */
        cmdlist = cmdlist[0].split(' ').map(|a| a.to_string()).collect();
    }

    Some(cmdlist)
}

fn qualify_process<'a>(cmdlist: &Vec<String>, parent_id: i32, map: &IndexMap<i32, Process>) -> Option<(String, u32, bool)> {
    if let Some(some) = map.get(&parent_id) {
        return Some((some.instance().into(), some.depth + 1, some.fork()));
    } else if cmdlist[0] == "pacwrap" {
        for idx in 0 .. cmdlist.len() {
            if cmdlist[idx].contains("-E") || cmdlist[idx].contains("run") || cmdlist[idx].contains("shell") {
                let mut pos = 0;

                for idx in 1 .. cmdlist.len() {
                    if cmdlist[idx].starts_with("-") || cmdlist[idx] == "run" || cmdlist[idx] == "shell" {
                        continue;
                    }

                    pos = idx;
                    break;
                }

                if pos == 0 {
                    break;
                }

                if let Some(var) = cmdlist.get(pos) {
                    return Some((var.into(), 1, false));
                }
            }
        }
    } else if cmdlist[0] == "bwrap" {
        for idx in 0 .. cmdlist.len() {
            if !cmdlist[idx].contains(&"--ro-bind") && !cmdlist[idx].contains("--bind") {
                continue;
            }

            if let Some(var) = cmdlist.get(idx + 1) {
                if var.starts_with(*CONTAINER_DIR) {
                    return Some((instance_from_path(var).into(), 1, true));
                }
            }
        }
    }

    None
}

fn instance_from_path(var: &str) -> &str {
    let length = CONTAINER_DIR.len();
    let var = var.split_at(length).1;

    var.find('/').map_or_else(|| var, |idx| var.split_at(idx).0)
}
