mod join_handler;
mod user_message_handler;

use crate::common::ChordMessage::{self};
use crate::common::{Message, ServerSignals, SERVER_FOLDER};
use crate::node_state::join_handler::handle_join;
use crate::node_state::user_message_handler::get_handler::handle_forwarded_get;
use crate::node_state::user_message_handler::handle_user_message;
use crate::node_state::user_message_handler::put_handler::handle_forwarded_put;
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use serde::Serialize;
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
    pub(crate) predecessor: Option<SocketAddr>,
    /// Time interval between gossip rounds.
    gossip_interval: Duration,
    sha: Sha256,
}

impl NodeState {
    pub fn new(ip: IpAddr, port: u16) -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let self_addr = SocketAddr::new(ip, port);
        let id = Sha256::digest(self_addr.to_string().as_bytes()).to_vec();

        handler.network().listen(Transport::Ws, self_addr)?;
        trace!("{:?}", id);

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
            predecessor: None,
            gossip_interval: Duration::from_secs(60),
            sha: Sha256::new(),
        };

        Ok(Self {
            handler,
            listener,
            config,
        })
    }

    pub fn connect_to(self, socket_addr: &str) -> Result<Self, io::Error> {
        let Self {
            handler,
            listener,
            config,
        } = self;
        let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));

        let serialized = bincode::serialize(&message).unwrap();

        let (endpoint, _) = handler.network().connect_sync(Transport::Ws, socket_addr)?;

        while handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
            trace!("Waiting for response...");
        }

        Ok(Self {
            handler,
            listener,
            config,
        })
    }

    pub fn run(self) {
        let Self {
            handler,
            listener,
            mut config,
        } = self;

        info!("start");
        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => {
                match net_event {
                    NetEvent::Message(endpoint, serialized) => {
                        //
                        let message = bincode::deserialize(serialized).unwrap();

                        match message {
                            Message::UserMessage(user_message) => {
                                trace!("Received user message");
                                handle_user_message(&handler, &mut config, endpoint, user_message);
                            }
                            Message::ChordMessage(server_message) => {
                                handle_server_message(&handler, &mut config, endpoint, server_message);
                            }
                        }
                    }
                    NetEvent::Connected(endpoint, result) => {
                        trace!("request from ep: {endpoint} connected: {result}");
                    }
                    NetEvent::Accepted(_, _) => {
                        trace!("Communication accepted");
                    }
                    NetEvent::Disconnected(endpoint) => {
                        handler.network().remove(endpoint.resource_id());
                    }
                }
            }
            NodeEvent::Signal(signal) => match signal {
                ServerSignals::ForwardMessage(endpoint, message) => {
                    trace!("Forwarding internal message");
                    forward_message(&handler, endpoint, message);
                }
                ServerSignals::SendMessageToUser(endpoint, message) => {
                    trace!("Forwarding message to user");
                    forward_message(&handler, endpoint, message);
                }
            },
        });
    }
}

fn saved_file_folder_exist(port: u16) -> bool {
    let folder_name = SERVER_FOLDER.to_string() + port.to_string().as_str() + "/" + SAVED_FILES;
    Path::new(&folder_name).exists()
}

fn forward_message(handler: &NodeHandler<ServerSignals>, endpoint: Endpoint, message: impl Serialize) {
    let output_data = bincode::serialize(&message).unwrap();
    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response");
    }
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

fn handle_server_message(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: ChordMessage,
) {
    match message {
        ChordMessage::AddPredecessor(addr) => config.predecessor = Some(addr), //todo send message to predecesor so that he knows his predecessor
        ChordMessage::SendStringMessage(mex, addr) => {
            let message = ChordMessage::Message(mex);
            trace!("Send message from {}, {}", addr, config.self_addr);
            let output_data = bincode::serialize(&message).unwrap();
            trace!("{endpoint}");
            handler.network().send(endpoint, &output_data);
        }
        ChordMessage::Join(addr) => {
            trace!("request from endpoint: {endpoint} ip: {addr}",);
            handle_join(handler, config, endpoint, addr);
            //Find the closest to that position
        }
        ChordMessage::Message(message) => {
            trace!("Message received from other peer: {message}");
        }
        ChordMessage::AddSuccessor(addr) => {
            //trace!("Add successor {endpoint}, {mex} {}", self.config.self_addr);
            config.finger_table.insert(0, addr);
            trace!("{}, {:?}", config.self_addr, config.finger_table);
        }
        ChordMessage::ForwardedJoin(addr) => {
            trace!("Forwarded join, joining {addr}");

            let (new_endpoint, _) = handler.network().connect(Transport::Ws, addr).unwrap();
            let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));
            handler
                .signals()
                .send(ServerSignals::ForwardMessage(new_endpoint, message));
        }
        ChordMessage::ForwardedPut(addr, file) => {
            trace!("Forwarded put");
            handle_forwarded_put(handler, config, addr, file);
        }
        ChordMessage::ForwardedGet(addr, key) => {
            trace!("Forwarded get");
            handle_forwarded_get(handler, config, addr, key);
        }
    }
}



