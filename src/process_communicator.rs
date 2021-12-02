pub static RELAY_CONNECTOR_PORT: i32 = 5555;
pub static RELAY_COLLECTOR_PORT: i32 = 5556;
pub static RELAY_PUBLISHER_PORT: i32 = 5557;

pub static ENGINE_PUB_SUB_ADDR: &str = "ipc:///tmp/webx-router-engine-pub-sub.ipc";
pub static ENGINE_PULL_PUSH_ADDR: &str = "ipc:///tmp/webx-router-engine-pull-push.ipc";
pub static ENGINE_PULL_PUSH_ADDR_FMT: &str = "ipc:///tmp/webx-router-engine-pull-push-{}.ipc";

static INPROC_PUB_SUB_ADDR: &str = "inproc://pub-sub-inproc";
pub static INPROC_APP_TOPIC: &str = "APP";
pub static INPROC_SESSION_TOPIC: &str = "SESSION";
pub static APPLICATION_SHUTDOWN_COMMAND: &str = "APP:SHUTDOWN";
pub static ENGINE_SESSION_START_COMMAND: &str = "SESSION:START";
pub static ENGINE_SESSION_END_COMMAND: &str = "SESSION:END";

pub enum SocketType {
    BIND,
    CONNECT,
}

pub struct ProcessCommunicator {
}

impl ProcessCommunicator {

    pub fn create_inproc_publisher(context: &zmq::Context, socket_type: SocketType) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::PUB).unwrap();
        socket.set_linger(0).unwrap();
        match socket_type {
            SocketType::BIND => {
                if let Err(error) = socket.bind(INPROC_PUB_SUB_ADDR) {
                    error!("Failed to bind inproc pub_sub to {}: {}", INPROC_PUB_SUB_ADDR, error);
                    return None;
                }
            },
            SocketType::CONNECT => {
                if let Err(error) = socket.connect(INPROC_PUB_SUB_ADDR) {
                    error!("Failed to connect inproc pub_sub to {}: {}", INPROC_PUB_SUB_ADDR, error);
                    return None;
                }
            }
        }

        Some(socket)
    }

    pub fn create_inproc_subscriber(context: &zmq::Context, topics: &[&str]) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::SUB).unwrap();
        if topics.len() == 0 {
            socket.set_subscribe(b"").unwrap();
        } else {
            for topic in topics {
                socket.set_subscribe(topic.as_bytes()).unwrap();
            }
        }
        socket.set_linger(0).unwrap();

        if let Err(error) = socket.connect(INPROC_PUB_SUB_ADDR) {
            error!("Failed to connect inproc SUB socket to {}: {}", INPROC_PUB_SUB_ADDR, error);
            return None;
        }

        Some(socket)
    }
}