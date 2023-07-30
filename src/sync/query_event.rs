#![allow(unused_variables)]

use alpm::{AnyQuestion, Question::*};

use crate::utils::prompt::prompt;

#[derive(Clone)]
pub struct QueryCallback;

pub fn questioncb(question: AnyQuestion, this: &mut QueryCallback) {
    match question.question() {
        Conflict(mut x) => {
            let reason = x.conflict().reason();
            let pkg_a = x.conflict().package1();
            let pkg_b = x.conflict().package2();

            let prompt_string = format!("Conflict between {} and {}; Remove {}?", pkg_a, pkg_b, pkg_b);
            let prompt =  prompt("->", prompt_string, false);
            if let Ok(_) = prompt {
                x.set_remove(true);
            }
        },
        Replace(x) => { 
            let prompt_string = format!("Replace package {} with {}?", x.oldpkg().name(), x.newpkg().name());
            let prompt =  prompt("->", prompt_string, false);
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
