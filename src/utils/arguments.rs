use std::env;
use std::collections::HashMap;
use std::rc::Rc;

use crate::utils;

use super::print_help_msg;

pub struct Arguments<'a, T> {
    prefix: String,
    runtime: Vec<Rc<str>>,
    targets: Vec<Rc<str>>,
    flags: HashMap<Rc<str>, (i8, i8)>,
    amalgamation: HashMap<Rc<str>, (i8, i8)>,
    value_map: HashMap<i8, &'a mut T>,
    count_map: HashMap<i8, &'a mut i32>,
    count: HashMap<i8, i32>,
    values: HashMap<i8, T>,
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
            set_index: 0,
            index: 0,
        }
    }

    pub fn parse_arguments(mut self) -> Arguments<'a, T> {
        let mut target = false;

        for string in env::args().skip(1) {
            let string: Rc<str> = string.into();
            
            if target {
                self.targets.push(string);
                target = false;
                continue;
            }

            match string {
                string if self.flags.contains_key(&string) => {
                    let key = self.flags.get(&string).unwrap(); 

                    if let Some(result) = self.values.remove(&key.1) {
                        if let Some(bool) = self.value_map.remove(&key.0) {
                            *bool = result;
                        }
                    }

                    if let Some(c) = self.count.get(&key.0) {
                        self.count.insert(key.0, c + 1); 
                    }
                },   
                string if string.starts_with(self.get_prefix()) => {
                    for amalgam in self.amalgamation.iter() {
                        for chars in string.chars() {
                            if chars != amalgam.0.chars().collect::<Vec<_>>()[0] {
                                continue;
                            }

                            if let Some(result) = self.values.remove(&amalgam.1.1) { 
                                if let Some(bool) = self.value_map.remove(&amalgam.1.0) {
                                    *bool = result;
                               }
                            }

                            if let Some(c) = self.count.get(&amalgam.1.0) {
                                self.count.insert(amalgam.1.0, c + 1); 
                            }
                        }
                    }
                },
                string if string.starts_with("-t") || string.starts_with("--target") => {
                    target = true;
                }, 
                _ => self.runtime.push(string),
            }
        }

        for idx in 0..self.index {
            if let Some(result) = self.count.remove(&idx) {
                if let Some(count) = self.count_map.remove(&idx) {
                    *count = result;
                }
            }
        }

        self
    }

    pub fn prefix(mut self, prfx: &str) -> Self {
        self.prefix.push_str(prfx.into());
        self
    }

    pub fn require_target(self, runtime: usize) -> Self { 
        if self.runtime.len() < runtime { utils::print_help_msg("Targets not specified. "); } 
        self
    }

    pub fn set(mut self, value: T) -> Self {
        self.values.insert(self.set_index, value); 
        self.set_index += 1; 
        self
    }

    pub fn increment(mut self) -> Self {
        self.index += 1;
        self
    }

    pub fn ignore(mut self, switch: &str) -> Self {
        self.flags.insert(switch.into(), (self.index, self.set_index));
        self.amalgamation.insert(switch.split_at(1).1.into(), (self.index, self.set_index));
        self.index+=1; 
        self
    }

    pub fn switch_big<'b>(mut self, big_switch: &str) -> Self { 
        self.flags.insert(big_switch.into(), (self.index, self.set_index)); 
        self
    }

    pub fn map<'b>(mut self, conditional: &'a mut T) -> Self { 
        self.value_map.insert(self.index, conditional);
        self
    }

    pub fn switch<'b>(mut self, switch: &str, big_switch: &str) -> Self { 
        self.flags.insert(switch.into(), (self.index, self.set_index));
        self.flags.insert(big_switch.into(), (self.index, self.set_index)); 
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
