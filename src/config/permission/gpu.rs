use std::fs::read_dir;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exec::args::ExecutionArgs;
use crate::config::{InsVars, Permission, permission::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GPU;

#[typetag::serde]
impl Permission for GPU {
    fn check(&self) -> Result<(),Error> {  
         if ! Path::new("/dev/").exists() {
            Err(Error::new("GPU", format!("/dev is inaccessible.")))?
        }

        Ok(())
    }
    
    fn register(&self, args: &mut  ExecutionArgs, vars: &InsVars) { 
        for e in read_dir("/dev/").unwrap() {
            match e {
                Ok(e) => {
                    let file = e.file_name();
                    let dev = file.to_str().unwrap();
                    match dev {
                        p if p.starts_with("nvidia") => args.dev(&format!("/dev/{}",dev)), 
                        p if p == "dri" => args.dev(&format!("/dev/{}",dev)),
                        &_ => {}
                    }                         
                }
                Err(_) => continue,
            }
        }
    }
}
