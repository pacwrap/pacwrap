use std::fmt::{Formatter, Debug};

pub struct ExecutionArgs {
    bind: Vec<String>,
    dev: Vec<String>,
    env: Vec<String>,
    dbus: Vec<String>
}

impl ExecutionArgs {
    pub fn new() -> Self {
        Self { 
            bind: Vec::new(), 
            dev: Vec::new(), 
            env: Vec::new(), 
            dbus: Vec::new() 
        }
    }

    pub fn dir(&mut self, dest: impl Into<String>)  {
        self.bind.push("--dir".into());
        self.bind.push(dest.into());
    }

    pub fn bind(&mut self, src: impl Into<String>, dest: impl Into<String>)  {
        self.bind.push("--bind".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn robind(&mut self, src: impl Into<String>, dest: impl Into<String>) {
        self.bind.push("--ro-bind".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn symlink(&mut self, src: impl Into<String>, dest: impl Into<String>) {
        self.bind.push("--symlink".into());
        self.bind.push(src.into());
        self.bind.push(dest.into());
    }

    pub fn env(&mut self, src: impl Into<String>, dest: impl Into<String>) {
        self.env.push("--setenv".into());
        self.env.push(src.into());
        self.env.push(dest.into());
    }

    pub fn dev(&mut self, src: impl Into<String> + Copy) {
        self.dev.push("--dev-bind-try".into());
        self.dev.push(src.into());
        self.dev.push(src.into());
    }

    pub fn dbus(&mut self, per: impl Into<String>, socket: impl Into<String>) {
        self.dbus.push(format!("--{}={}", per.into(), socket.into()));
    }

    pub fn push_env(&mut self, src: impl Into<String>) { 
        self.env.push(src.into()); 
    }

    pub fn get_bind(&self) -> &Vec<String> { 
        &self.bind 
    }

    pub fn get_dev(&self) -> &Vec<String> { 
        &self.dev 
    }

    pub fn get_env(&self) -> &Vec<String> { 
        &self.env
    }

    pub fn get_dbus(&self) -> &Vec<String> { 
        &self.dbus 
    }
}

impl Debug for ExecutionArgs {
    fn fmt(&self, fmter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmter, "bind: {:?}", self.bind)?;  
        writeln!(fmter, "env:  {:?}", self.env)?;

        if self.dev.len() > 0 {
            writeln!(fmter, "dev:  {:?}", self.dev)?; 
        }

        if self.dbus.len() > 0 {
            writeln!(fmter, "dbus: {:?}", self.dbus)?;
        }

        Ok(())
    }
}
