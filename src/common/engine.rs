use std::process::Child;

pub struct Engine {
    process: Child,
    ipc: String,
}

impl Engine {

    pub fn new(process: Child, ipc: String) -> Self {
        Self {
            process,
            ipc,
        }
    }

    pub fn process(&mut self) -> &mut Child {
        return &mut self.process;
    }

    pub fn ipc(&self) -> &str {
        return &self.ipc;
    }
}
