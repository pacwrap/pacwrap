use std::env;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::utils;

use super::print_help_msg;

pub struct Arguments<'a, T> {
    prefix: String,
    runtime: Vec<Rc<str>>,
    targets: Vec<Rc<str>>,
    ignored: HashSet<Rc<str>>,
    flags: HashMap<Rc<str>, (i8, i8)>,
    amalgamation: HashMap<Rc<str>, (i8, i8)>,
    value_map: HashMap<i8, &'a mut T>,
    count_map: HashMap<i8, &'a mut i32>,
    count: HashMap<i8, i32>,
    values: HashMap<i8, T>,
    assume_target: bool,
    set_index: i8,
    index: i8,
}

impl<'a, T> Arguments<'a, T> where T: Copy {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            prefix: String::new(), 
            runtime: Vec::new(),
            flags: HashMap::new(),
            amalgamation: HashMap::new(),
            value_map: HashMap::new(),
            count_map: HashMap::new(),
            count: HashMap::new(),
            values: HashMap::new(),
            ignored: HashSet::new(),
            assume_target: false,
            set_index: 0,
            index: 0,
        }
    }

    pub fn parse_arguments(mut self) -> Arguments<'a, T> { 
        for string in env::args().skip(1) {
            let string: Rc<str> = string.into(); 

            if let Some(_) = self.ignored.get(&string) {
                continue;
            }

            match string {
                string if self.flags.get(&string).is_some() => {
                    let key = self.flags.get(&string).unwrap(); 

                    if let Some(result) = self.values.remove(&key.1) {
                        if let Some(bool) = self.value_map.remove(&key.0) {
                            *bool = result;
                        }
                    }

                    if let Some(count) = self.count_map.get_mut(&key.0) {
                        **count = **count + 1;
                    }
                },   
                string if string.starts_with(self.get_prefix()) => {      
                    for amalgam in self.amalgamation.iter() { 
                        for char in string.chars() { 
                            if char != amalgam.0.chars().collect::<Vec<_>>()[0] || char == '-' {
                                if char == 't' {
                                    self.assume_target = true;
                                }

                                continue;
                            }

                            if let Some(result) = self.values.remove(&amalgam.1.1) { 
                                if let Some(bool) = self.value_map.remove(&amalgam.1.0) {
                                    *bool = result;
                                }
                            }
                            
                            if let Some(count) = self.count_map.get_mut(&amalgam.1.0) { 
                                **count = **count + 1;
                            }
                        }
                    }
                },
                string if string.starts_with("-t") || string.starts_with("--target") => {
                    self.assume_target = true;
                }, 
                _ => { 
                    if self.assume_target {
                        self.targets.push(string);
                        self.assume_target = false;
                    } else {
                        self.runtime.push(string);
                    }
                }
            }
        }

        self
    }

    pub fn prefix(mut self, prfx: &str) -> Self {
        self.prefix.push_str(prfx.into());
        self
    }

    #[allow(dead_code)]
    pub fn validate(self, expected: usize) -> Self { 
        if self.runtime.len() > expected {
            invalid();
        }

        self
    }

    pub fn require_target(self, expected: usize) -> Self { 
        if self.targets.len() < expected {
            let noun = if expected > 1 { "Targets" } else { "Target" };

            utils::print_help_msg(format!("{noun} not specified.")); 
        }

        self
    }

    pub fn set(mut self, value: T) -> Self {
        self.values.insert(self.set_index, value); 
        self.set_index += 1; 
        self
    }

    pub fn push(mut self) -> Self {
        self.index += 1;
        self
    }

    pub fn assume_target(mut self) -> Self {
        self.assume_target = true;
        self
    }

    pub fn ignore(mut self, switch: &str) -> Self {
        self.ignored.insert(switch.into());
        self.ignored.insert(switch.split_at(1).1.into());
        self
    }

    pub fn long<'b>(mut self, switch: &str) -> Self { 
        self.flags.insert(switch.into(), (self.index, self.set_index)); 
        self
    }

    pub fn map<'b>(mut self, conditional: &'a mut T) -> Self { 
        self.value_map.insert(self.index, conditional);
        self
    }

    pub fn short<'b>(mut self, switch: &str) -> Self { 
        self.flags.insert(switch.into(), (self.index, self.set_index));
        self.amalgamation.insert(switch.split_at(1).1.into(), (self.index, self.set_index));
        self
    }

    pub fn count<'b>(mut self, count: &'a mut i32) -> Self {  
        self.count_map.insert(self.index, count);
        self.count.insert(self.index, 0);
        self
    }

    pub fn targets(&self) -> &Vec<Rc<str>> { &self.targets }
    pub fn get_runtime(&self) -> &Vec<Rc<str>> { &self.runtime }
    pub fn get_prefix(&self) -> &String { &self.prefix }
}

pub fn print_version() {
    let info=concat!("Copyright (C) 2023 Xavier R.M.\n\n",
                     "Website: https://git.sapphirus.org/pacwrap\n",
                     "Github: https://github.com/sapphirusberyl/pacwrap\n\n",
                     "This program may be freely redistributed under\n",
                     "the terms of the GNU General Public License v3.\n");

    println!("{} {}\n{info}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

pub fn invalid() {
    let mut ar = String::new();
    for arg in env::args().skip(1).collect::<Vec<_>>().iter() {
        if arg == "--fake-chroot" {
            continue;
        }
        ar.push_str(&format!("{} ", &arg));
    } 
    ar.truncate(ar.len()-1);
    print_help_msg(&format!("Invalid arguments -- '{}'", ar));
}
