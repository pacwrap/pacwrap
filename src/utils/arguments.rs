use std::env;
use std::collections::HashMap;

use crate::utils;

use super::print_help_msg;

pub struct Arguments<'a> {
    prefix: String,
    runtime: Vec<String>,
    targets: Vec<String>,
    prefixes: HashMap<String, i8>,
    amalgamation: HashMap<String, i8>,
    bool_map: HashMap<i8, &'a mut bool>,
    count_map: HashMap<i8, &'a mut i32>,
    count: HashMap<i8, i32>,
    bools: HashMap<i8, bool>,
    index: i8
}

impl<'a> Arguments<'a> {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            prefix: String::new(), 
            runtime: Vec::new(),
            prefixes: HashMap::new(),
            amalgamation: HashMap::new(),
            bool_map: HashMap::new(),
            count_map: HashMap::new(),
            count: HashMap::new(),
            bools: HashMap::new(),
            index: 0
        }
    }

    pub fn parse_arguments(mut self) -> Arguments<'a> {
        let mut target = false;

        for string in env::args().skip(1) {
            if target {
                self.targets.push(string);
                target = false;
                continue;
            }

            match string {
                string if self.prefixes.contains_key(&string) => {
                    let key = self.prefixes.get(&string).unwrap(); 
                    self.bools.insert(key.clone(), true);
                    if let Some(c) = self.count.get(key) {
                        let count = c + 1;
                        self.count.insert(key.clone(), count); 
                    }

                },   
                string if string.starts_with(self.get_prefix()) => {
                    for amalgam in self.amalgamation.iter() {
                        for chars in string.chars() {
                        if chars != amalgam.0.chars().collect::<Vec<_>>()[0] {
                            continue;
                        }
             
                        self.bools.insert(amalgam.1.clone(), true);

                        if let Some(c) = self.count.get(amalgam.1) {
                            let count = c + 1;
                                self.count.insert(amalgam.1.clone(), count); 
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

        for idx in 0..self.index {
            if let Some(result) = self.bools.remove(&idx) {
                if let Some(bool) = self.bool_map.remove(&idx) {
                    *bool = result;
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

    pub fn ignore(mut self, switch: &str) -> Self {
        self.prefixes.insert(switch.into(), self.index);
        self.amalgamation.insert(switch.split_at(1).1.to_string(), self.index);
        self.index+=1; 
        self
    }

    pub fn switch_big<'b>(mut self, big_switch: &str, conditional: &'a mut bool) -> Self { 
        self.bool_map.insert(self.index, conditional);
        self.prefixes.insert(big_switch.into(), self.index); 
        self.index+=1;
        self
    }

    pub fn switch<'b>(mut self, switch: &str, big_switch: &str, conditional: &'a mut bool) -> Self { 
        self.bool_map.insert(self.index, conditional);
        self.prefixes.insert(switch.into(), self.index);
        self.prefixes.insert(big_switch.into(), self.index); 
        self.amalgamation.insert(switch.split_at(1).1.to_string(), self.index);
        self.index+=1;
        self
    }

    pub fn count<'b>(mut self, count: &'a mut i32) -> Self {  
        let index = self.index-1;
        self.count_map.insert(index, count);
        self.count.insert(index, 0);
        self
    }

    pub fn targets(&self) -> &Vec<String> { &self.targets }
    pub fn get_runtime(&self) -> &Vec<String> { &self.runtime }
    pub fn get_prefix(&self) -> &String { &self.prefix }
}

pub fn invalid() {
    let mut ar = String::new();
    for arg in env::args().skip(1).collect::<Vec<_>>().iter() {
        ar.push_str(&format!("{} ", &arg));
    } 
    ar.truncate(ar.len()-1);
    print_help_msg(&format!("Invalid arguments -- '{}'", ar));
}
