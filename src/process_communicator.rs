pub static RELAY_CONNECTOR_PORT: i32 = 5555;
pub static RELAY_COLLECTOR_PORT: i32 = 5556;
pub static RELAY_PUBLISHER_PORT: i32 = 5557;

pub static ENGINE_SUBSCRIBER_ADDR: &str = "ipc:///tmp/webx-router-engine-pub-sub.ipc";
pub static SHUTDOWN_COMMAND: &str = "SHUTDOWN";

static SHUTDOWN_ADDR: &str = "inproc://shutdown";

pub struct ProcessCommunicator {
}

impl ProcessCommunicator {

    pub fn create_inproc_publisher(context: &zmq::Context) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::PUB).unwrap();
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.bind(SHUTDOWN_ADDR) {
            error!("Failed to bind shutdown publisher to {}: {}", SHUTDOWN_ADDR, error);
            return None;
        }

        Some(socket)
    }

    pub fn create_inproc_subscriber(context: &zmq::Context) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::SUB).unwrap();
        socket.set_subscribe(b"").unwrap();
        socket.set_linger(0).unwrap();

        if let Err(error) = socket.connect(SHUTDOWN_ADDR) {
            error!("Failed to connect inproc SUB socket to {}: {}", SHUTDOWN_ADDR, error);
            return None;
        }

        Some(socket)
    }
}