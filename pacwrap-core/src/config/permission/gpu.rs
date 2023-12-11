use std::fs::read_dir;
use std::path::Path;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{exec::args::ExecutionArgs,
    config::{Permission, permission::*},
    config::permission::{Condition::Success, PermError::Fail}};

lazy_static! {
    static ref GPU_DEV: Vec<String> = populate_dev();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GPU;

#[typetag::serde]
impl Permission for GPU {
    fn check(&self) -> Result<Option<Condition>, PermError> {  
        if ! Path::new("/dev").exists() {
            Err(Fail(format!("/dev is inaccessible.")))?
        }

        if GPU_DEV.len() == 0 {
            Err(Fail(format!("No graphics devices are available.")))? 
        }

        Ok(Some(Success))
    }
    
    fn register(&self, args: &mut  ExecutionArgs) { 
        for dev in GPU_DEV.iter() {
            args.dev(dev);
        }
    }

    fn module(&self) -> &'static str {
        "GPU"
    }
}

fn populate_dev() -> Vec<String> {
    let mut vec: Vec<String> = Vec::new();
    if let Ok(dir) = read_dir("/dev") {
        for f in dir {
            if let Ok(f) = f {
                let file = f.file_name();
                let dev = file.to_str().unwrap();
                if dev.starts_with("nvidia") || dev == "dri" {
                    vec.push(format!("/dev/{}",dev));
                }
            }
        }
    }
    vec
}
