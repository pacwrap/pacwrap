use std::env;
use std::collections::HashMap;

use crate::utils;

#[derive(Clone)]
pub struct Arguments {
    prefix: String,
    switch: String,
    runtime: Vec<String>,
    targets: Vec<String>,
    target: usize,
    argument_map: HashMap<String, String>
}

impl Arguments {
    pub fn new(amt: usize, arg: impl Into<String>, arg_map: HashMap<String, String>) -> Self {
        let mut arguments = Self { 
            prefix: arg.into(), 
            switch: String::new(), 
            targets: Vec::new(), 
            runtime: Vec::new(),
            target: amt,
            argument_map: arg_map,
        };
        arguments.parse_arguments();
        return arguments;
    }

    fn parse_arguments(&mut self) {
        for string in env::args().skip(1) {
            match string {
                string if self.argument_map.contains_key(&string) => self.append_switch(self.argument_map.get(&string).unwrap().clone()),  
                string if string.starts_with(self.get_prefix()) => self.append_switch(&string[self.get_prefix().len()..]),  
                _ => { if ! self.target_reached() { self.targets.push(string); } else { self.runtime.push(string); } },
            }
        }

        if ! self.target_reached() { utils::print_help_msg("Targets not specified. "); }
    }

    fn append_switch(&mut self, arg: impl Into<String>) { self.switch = self.switch.clone()+&arg.into(); }
    fn target_reached(&self) -> bool { self.targets.len() == self.target }
    pub fn get_runtime(&self) -> &Vec<String> { &self.runtime }
    pub fn get_switch(&self) -> &String { &self.switch }
    pub fn get_prefix(&self) -> &String { &self.prefix }
    pub fn get_targets(&self) -> &Vec<String> { &self.targets }
}
