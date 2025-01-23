mod handlers;

use crate::common::ChordMessage::{self};
use crate::common::{Message, ServerSignals, SERVER_FOLDER};

use crate::node_state::handlers::event::{net_handler, signal_handler};
use chrono::{DateTime, TimeDelta, Utc};
use crate::node_state::handlers::event::{handle_net_event, handle_server_signal};
use message_io::network::{Endpoint, SendStatus, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};
use tracing::{error, info, trace};

const SAVED_FILES: &str = "saved_files.txt";
const ID_BYTES: usize = 32;
const FINGER_TABLE_SIZE: usize = 255;

const MAXIMUM_DURATION: Duration = Duration::from_secs(320);

const HEARTBEAT_TIMEOUT: TimeDelta = TimeDelta::seconds(30);

const HEART_BEAT: Duration = Duration::from_secs(5);
pub struct NodeState {
    handler: NodeHandler<ServerSignals>,
    listener: NodeListener<ServerSignals>,
    config: NodeConfig,
}

pub struct NodeConfig {
    pub(crate) id: Vec<u8>,
    /// The node's own address.
    pub(crate) self_addr: SocketAddr,
    /// Maps hashed key to file name.
    pub(crate) saved_files: HashMap<String, String>,
    ///List of successors node
    pub(crate) finger_table: Vec<SocketAddr>,

    pub(crate) successors_cache: Vec<SocketAddr>,

    pub(crate) last_modified: DateTime<Utc>,

    pub(crate) known_endpoints_ws: HashMap<SocketAddr, Endpoint>,

    pub(crate) known_endpoints_udp: HashMap<SocketAddr, Endpoint>,

    pub(crate) predecessor: Option<SocketAddr>,
    /// Time interval between gossip rounds.
    gossip_interval: Duration,
}

impl NodeState {
    /// Creates a new instance of the struct, initializing the network handler, listener, and node configuration.
    ///
    /// This function performs the following operations:
    /// 1. Initializes a network handler and a listener by calling `node::split()`.
    /// 2. Configures the handler to listen for WebSocket (`Ws`) and UDP (`Udp`) connections on the specified address and port.
    /// 3. Calculates a unique identifier for the node based on the `SocketAddr` (IP and port) using a SHA-256 hash.
    /// 4. Ensures the folder structure for storing saved files exists, creating it if necessary.
    /// 5. Loads previously saved files from the storage folder, or initializes an empty list if no saved files are found.
    /// 6. Constructs the node configuration, including the finger table and other parameters, and initializes the struct.
    ///
    /// # Parameters
    /// - `ip`: An `IpAddr` (e.g., `Ipv4Addr` or `Ipv6Addr`) representing the IP address where the node will listen for incoming connections.
    /// - `port`: A `u16` representing the port number where the node will listen for incoming connections.
    ///
    /// # Returns
    /// - `Ok(Self)`: Returns an instance of the struct if the initialization is successful.
    /// - `Err(io::Error)`: Returns an I/O error if the handler fails to bind to the specified address and port.
    ///
    ///
    ///
    /// # Example
    /// ```rust
    /// use std::net::{IpAddr, Ipv4Addr};
    /// use std::io;
    ///
    ///     use DHTchord::node_state::{NodeConfig, NodeState};
    /// let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    ///     let port = 8080;
    ///
    ///     let node = NodeState::new(ip, port)?;
    /// ```
    ///
    ///
    /// # Key Components in the Configuration
    /// - `id`: A unique identifier for the node, calculated using SHA-256 of the node's address.
    /// - `self_addr`: The address (`SocketAddr`) where the node is listening.
    /// - `saved_files`: A collection of files loaded from the storage folder, used for file management.
    /// - `finger_table`: An empty vector representing the initial finger table for the node (used in distributed systems like Chord).
    /// - `gossip_interval`: The interval for gossip-based communication, set to 5 seconds.

    pub fn new(ip: IpAddr, port: u16) -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let self_addr = SocketAddr::new(ip, port);
        let id = Sha256::digest(self_addr.to_string().as_bytes()).to_vec();

        handler.network().listen(Transport::Ws, self_addr)?;
        handler.network().listen(Transport::Udp, self_addr)?;

        if !saved_file_folder_exist(port) {
            if let Err(x) = create_saved_file_folder(port) {
                error!("ERROR {:?} trying to create the file for saved_files record", x);
            }
        }

        let saved_files = load_from_folder(port).unwrap_or_default();

        let config = NodeConfig {
            id,
            self_addr,
            saved_files,
            finger_table: vec![],
            successors_cache: vec![],
            last_modified: Utc::now(),
            known_endpoints_ws: Default::default(),
            known_endpoints_udp: Default::default(),
            predecessor: None,
            gossip_interval: Duration::from_secs(5),
        };

        Ok(Self {
            handler,
            listener,
            config,
        })
    }

    /// Connects to a remote node in the Chord network and starts the main event loop for the node.
    ///
    /// # Parameters
    /// - `self`: Consumes the instance of the struct to perform the connection and event loop. This ensures the instance cannot be reused afterward.
    /// - `socket_addr`: A `SocketAddr` representing the address of the remote node to connect to.
    ///
    ///
    /// # Example
    /// ```rust
    /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// use DHTchord::node_state::NodeState;
    ///
    /// let node = NodeState::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080).unwrap();
    /// let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 8080);
    /// node.connect_and_run(remote_addr);
    ///
    /// ```
    ///
    /// # Notes
    /// - The `self.run()` call at the end of the function starts the node's main event loop. Ensure that the `run` method is implemented to handle the Chord protocol's core logic.
    /// - The `finger_table_map` is updated with the remote node's address and endpoint, facilitating routing and communication within the Chord network.
    ///
    pub fn connect_and_run(mut self, socket_addr: SocketAddr) {
        let message = Message::ChordMessage(ChordMessage::Join(self.config.self_addr));

        let serialized = bincode::serialize(&message).unwrap();

        let (endpoint, _) = handler.network().connect_sync(Transport::Ws, socket_addr).unwrap();

        self.config.known_endpoints_ws.insert(socket_addr, endpoint);

        self.config.last_modified = Utc::now();

        while self.handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
            trace!("Waiting for response...");
        }

        self.run();
    }

    /// Starts the main event loop for the node, handling network events and periodic stabilization tasks.
    ///
    /// # Parameters
    /// - `self`: Consumes the struct instance to initiate the event loop. The instance is no longer accessible after this call.
    ///
    /// # Behavior
    /// - **Signal Management:** A stabilization signal is scheduled at regular intervals defined by `self.config.gossip_interval`.
    /// - **Event Processing:** Listens indefinitely for events, ensuring that the node remains active in the Chord network and responds to network or scheduled events.
    /// - **Handlers:** Delegates event-specific logic to:
    ///   - `handle_net_event` for network-related tasks.
    ///   - `handle_server_signal` for signal-related tasks.
    ///
    /// # Example
    /// ```rust
    /// use std::net::{IpAddr, Ipv4Addr};
    /// use DHTchord::node_state::NodeState;
    /// let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    /// let port = 8080;
    ///
    /// let node = NodeState::new(ip, port).unwrap();
    /// node.run();
    ///
    /// ```
    pub fn run(mut self) {
        info!("start");

        self.handler
            .signals()
            .send_with_timer(ServerSignals::Stabilization(), config.gossip_interval);

        self.handler
            .signals()
            .send_with_timer(ServerSignals::HeartBeat(), HEART_BEAT);

        self.listener.for_each(move |event| match event {
            NodeEvent::Network(event) => handle_net_event(&self.handler, &mut self.config, event),
            NodeEvent::Signal(signal) => handle_server_signal(&self.handler, &mut self.config, signal),
        });
    }
}
fn saved_file_folder_exist(port: u16) -> bool {
    let folder_name = SERVER_FOLDER.to_string() + port.to_string().as_str() + "/" + SAVED_FILES;
    Path::new(&folder_name).exists()
}

///function to save the hashmap of key-file name
fn create_saved_file_folder(port: u16) -> io::Result<()> {
    if !Path::new(SERVER_FOLDER).exists() {
        fs::create_dir(SERVER_FOLDER)?;
    }
    let folder_name = SERVER_FOLDER.to_string() + port.to_string().as_str() + "/";

    if !Path::new(&folder_name).exists() {
        fs::create_dir(&folder_name)?;
    }

    let file_path = folder_name + SAVED_FILES;
    let mut file = File::create(&file_path)?;
    file.flush()?;
    Ok(())
}

fn load_from_folder(port: u16) -> io::Result<HashMap<String, String>> {
    let file_path = SERVER_FOLDER.to_string() + port.to_string().as_str() + "/" + SAVED_FILES;
    let mut saved_files = HashMap::new();

    let file = File::open(&file_path)?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Some((key, value)) = line?.split_once(':') {
            saved_files.insert(key.to_string(), value.to_string());
        }
    }

    Ok(saved_files)
}
