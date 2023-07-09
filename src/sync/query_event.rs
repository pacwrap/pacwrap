#![allow(unused_variables)]

use alpm::{AnyQuestion, Question::*};
use console::style;

use crate::utils::prompt::prompt;

#[derive(Clone)]
pub struct QueryCallback;

impl QueryCallback {
    pub fn new() -> Self {
        Self
    }
}

pub fn questioncb(question: AnyQuestion, this: &mut QueryCallback) {
    match question.question() {
        Conflict(mut x) => {
            let reason = x.conflict().reason();
            let pkg_a = x.conflict().package1();
            let pkg_b = x.conflict().package2();

            let prompt_string = format!("Conflict between {} and {}: {}. Remove package?", pkg_a, pkg_b, reason);
            let prompt =  prompt("->", style(&prompt_string), false);
            if let Ok(_) = prompt {
                x.set_remove(true);
            }
        },
        Replace(x) => { 
            let prompt_string = format!("Replace package {} with {}?", x.oldpkg().name(), x.newpkg().name());
            let prompt =  prompt("->", style(&prompt_string), false);
            if let Ok(_) = prompt {
                x.set_replace(true);
            }
        },
        Corrupted(x) => (),
        RemovePkgs(x) => (),
        ImportKey(x) => (),
        _ => ()
    }
}
