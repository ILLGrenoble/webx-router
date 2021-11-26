use std::thread;

pub struct Publisher {
    running: bool,
}

impl Publisher {

    pub fn new() -> Self {
        Self {
            running: true
        }
    }

    pub fn run(&self, context: & zmq::Context) {
        thread::spawn(move || {
            self.mainLoop();
        });
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    fn mainLoop(&self) {
        while self.running {

        }
    }

}
