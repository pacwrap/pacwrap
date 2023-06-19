#[derive(Debug)]
pub struct ExecutionArgs {
    bind: Vec<String>,
    dev: Vec<String>,
    envir: Vec<String>,
    dbus: Vec<String>
}


impl ExecutionArgs {

    pub fn new() -> Self {
        Self { bind: Vec::new(), dev: Vec::new(), envir: Vec::new(), dbus: Vec::new() }
    }

    pub fn get_bind(&self) -> &Vec<String> { &self.bind }
    pub fn get_dev(&self) -> &Vec<String> { &self.dev }
    pub fn get_env(&self) -> &Vec<String> { &self.envir }
    pub fn get_dbus(&self) -> &Vec<String> { &self.dbus }
    pub fn push_env(&mut self, src: impl Into<String>) { self.envir.push(src.into()); }

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
        self.envir.push("--setenv".into());
        self.envir.push(src.into());
        self.envir.push(dest.into());
    }

    pub fn dev(&mut self, src: impl Into<String>) {
        let dev = src.into();
        self.dev.push("--dev-bind-try".into());
        self.dev.push(dev.clone());
        self.dev.push(dev.clone());
    }

    pub fn dbus(&mut self, per: impl Into<String>, socket: impl Into<String>) {
        self.dbus.push("--".to_owned()+&per.into()+"="+&socket.into());
    }

}
