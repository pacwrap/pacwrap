use std::env;
use std::collections::HashMap;

use crate::utils;

pub struct Arguments<'a> {
    prefix: String,
    runtime: Vec<String>,
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

        for string in env::args().skip(1) {
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
                _ => self.runtime.push(string),
            }
        }

        for idx in 0..self.index {
            if let Some(result) = self.count.remove(&idx) {
                let count = self.count_map.remove(&idx).unwrap(); 
                *count = result;
            }
        }

        for idx in 0..self.index {
            if let Some(result) = self.bools.remove(&idx) {
                let bool = self.bool_map.remove(&idx).unwrap(); 
                *bool = result;
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

    pub fn get_runtime(&self) -> &Vec<String> { &self.runtime }
    pub fn get_prefix(&self) -> &String { &self.prefix }
}
