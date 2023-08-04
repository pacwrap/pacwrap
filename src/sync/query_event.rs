use alpm::{AnyQuestion, Question::*};

use crate::utils::prompt::prompt;

#[derive(Clone)]
pub struct QueryCallback;

pub fn questioncb(question: AnyQuestion, _: &mut QueryCallback) {
    match question.question() {
        Conflict(mut x) => {
            let reason = x.conflict().reason();
            let pkg_a = x.conflict().package1();
            let pkg_b = x.conflict().package2();
            let prompt_string = format!("Conflict between {} and {} ({}); Remove {}?", pkg_a, pkg_b, reason, pkg_b);
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
        Corrupted(mut x) => {
            let prompt_string = format!("'{}': {}. Remove?", x.filepath(), x.reason());
            let prompt =  prompt("->", prompt_string, false);
            if let Ok(_) = prompt {
                x.set_remove(true);
            }
        },
        ImportKey(mut x) => {
            let key = x.key(); 
            let prompt_string = format!("Import PGP key {}, \"{} <{}>\", created: {}", 
                key.fingerprint(), 
                key.name(), 
                key.email(), 
                key.created());
            let prompt =  prompt("->", prompt_string, false);
            if let Ok(_) = prompt {
                x.set_import(true);
            } 
        },
        InstallIgnorepkg(mut x) => {
            let prompt_string = format!("Package {} is ignored. Install anyway?", x.pkg().name());
            let prompt =  prompt("->", prompt_string, true);
            if let Ok(_) = prompt {
                x.set_install(true);
            } 
        },
        _ => ()
    }
}
