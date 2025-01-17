mod handlers;

use crate::common::ChordMessage::{self};
use crate::common::{Message, ServerSignals, SERVER_FOLDER};

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
//TODO always order finger table, check if it works

const SAVED_FILES: &str = "saved_files.txt";
const ID_BYTES: usize = 32;
const FINGER_TABLE_SIZE: usize = 4;

const MAXIMUM_DURATION: Duration = Duration::from_secs(320);

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

    pub(crate) finger_table_map: HashMap<SocketAddr, Endpoint>,

    pub(crate) predecessor: Option<SocketAddr>,
    /// Time interval between gossip rounds.
    gossip_interval: Duration,
}

impl NodeState {
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
            finger_table_map: Default::default(),
            predecessor: None,
            gossip_interval: Duration::from_secs(5),
        };

        Ok(Self {
            handler,
            listener,
            config,
        })
    }

    pub fn personalized_id_test(&mut self, new_test_id: Vec<u8>) {
        self.config.id = new_test_id;
    }

    pub fn connect_and_run(mut self, socket_addr: SocketAddr) {
        let message = Message::ChordMessage(ChordMessage::Join(self.config.self_addr));

        let serialized = bincode::serialize(&message).unwrap();

        let (endpoint, _) = self.handler.network().connect_sync(Transport::Ws, socket_addr).unwrap();

        self.config.finger_table_map.insert(socket_addr, endpoint);

        while self.handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
            trace!("Waiting for response...");
        }

        self.run();
    }

    pub fn run(mut self) {
        info!("start");

        self.handler
            .signals()
            .send_with_timer(ServerSignals::Stabilization(), self.config.gossip_interval);

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
