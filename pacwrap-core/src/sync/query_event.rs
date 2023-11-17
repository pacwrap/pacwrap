use std::path::Path;

use alpm::{AnyQuestion, Question::*};

use crate::utils::prompt::prompt;

pub fn questioncb(question: AnyQuestion, _: &mut ()) {
    match question.question() {
        Conflict(mut x) => {
            let pkg_a = x.conflict().package1();
            let pkg_b = x.conflict().package2();
            let prompt_string = format!("Conflict between {pkg_a} and {pkg_b}; Remove {pkg_b}?");
            
            if let Ok(_) = prompt("->", prompt_string, false) {
                x.set_remove(true);
            }
        },
        Replace(x) => { 
            let old = x.oldpkg().name();
            let new = x.newpkg().name();
            let prompt_string = format!("Replace package {old} with {new}?");
            
            if let Ok(_) = prompt("->", prompt_string, false) {
                x.set_replace(true);
            }
        },
        Corrupted(mut x) => {
            let filepath = x.filepath();
            let filename = Path::new(filepath).file_name().unwrap().to_str().unwrap();
            let reason = x.reason();
            let prompt_string = format!("'{filename}': {reason}. Remove package?");

            if let Ok(_) = prompt("::", prompt_string, true) {
                x.set_remove(true);
            }
        },
        ImportKey(mut x) => {
            let key = x.key();
            let fingerprint = key.fingerprint();
            let email = key.email();
            let name = key.name();
            let prompt_string = format!("Import key {fingerprint},\"{name} <{email}>\" to keyring?");
            
            if let Ok(_) = prompt("->", prompt_string, true) {
                x.set_import(true);
            } 
        },
        //TODO: Implement these questions.
        RemovePkgs(_) => (),
        SelectProvider(_) => (),
        InstallIgnorepkg(_) => (),
    }
}
