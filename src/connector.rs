use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

static CONNECTOR_PORT: i32 = 5555;
static COLLECTOR_PORT: i32 = 5556;
static PUBLISHER_PORT: i32 = 5557;


pub struct Connector {
    running: Arc<AtomicBool>,
}

impl Connector {

    pub fn new() -> Self {
        Self{
            running: Arc::new(AtomicBool::new(true))
        }
    }

    pub fn init(&self) {
        let running = self.running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");
    }

    pub fn run(&self, socket_timeout_ms: i32) {
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::REP).unwrap();

        socket.set_linger(0).unwrap();
        socket.set_rcvtimeo(socket_timeout_ms).unwrap();

        let address = format!("tcp://*:{}", CONNECTOR_PORT);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind to {}: {}", address, error);
            return;
        }

        while self.is_running() {
            let mut msg = zmq::Message::new();
            let mut send_required = false;

            // Read next message
            if let Err(error) = socket.recv(&mut msg, 0) {
                if self.is_running() {
                    error!("Failed to received message: {}", error);
                }

            } else {
                info!("Got message {}", msg.as_str().unwrap());
                send_required = true;

                let message_text = msg.as_str().unwrap();

                // Check for comm message
                if msg.len() == 4 && message_text == "comm" {
                    // Send response
                    if let Err(error) = socket.send(format!("{},{}", PUBLISHER_PORT, COLLECTOR_PORT).as_str(), 0) {
                        error!("Failed to send comm message: {}", error);
                    }
                    send_required = false;
                }
            }

            // Check for shutdown
            if self.is_running() {
                // If send needed then send empty message
                if send_required {
                    let empty_message = zmq::Message::new();
                    if let Err(error) = socket.send(empty_message, 0) {
                        error!("Failed to send empty message: {}", error);
                    }
                }

            } else {
                self.stop();
            }

        }
    }

    fn stop(&self) {
        info!("Stopping WebX Router...");
        self.running.store(false, Ordering::SeqCst);

    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

}
