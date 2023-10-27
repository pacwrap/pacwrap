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
            let prompt_string = format!("Conflict between {pkg_a} and {pkg_b}; Remove {pkg_b}?");
            let prompt =  prompt("->", prompt_string, false);
            
            if let Ok(_) = prompt {
                x.set_remove(true);
            }
        },
        Replace(x) => { 
            let old = x.oldpkg().name();
            let new = x.newpkg().name();
            let prompt_string = format!("Replace package {old} with {new}?");
            let prompt =  prompt("->", prompt_string, false);
            
            if let Ok(_) = prompt {
                x.set_replace(true);
            }
        },
        Corrupted(mut x) => {
            let prompt_string = format!("Remove corrupted package {}?", x.filepath());
            let prompt =  prompt("->", prompt_string, true);
            
            if let Ok(_) = prompt {
                x.set_remove(true);
            }
        },
        ImportKey(mut x) => {
            let key = x.key();
            let fingerprint = key.fingerprint();
            let email = key.email();
            let name = key.name();
            let prompt_string = format!("Import key {fingerprint},\"{name} <{email}>\" to keyring?");
            let prompt =  prompt("->", prompt_string, true);
            
            if let Ok(_) = prompt {
                x.set_import(true);
            } 
        },
        RemovePkgs(_) => (),
        SelectProvider(_) => (),
        InstallIgnorepkg(_) => (),
    }
}
