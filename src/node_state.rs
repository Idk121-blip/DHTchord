use crate::common;
use crate::common::ChordMessage::{self};
use crate::common::{Message, ServerSignals, ServerToUserMessage, UserMessage};
use crate::errors::{GetError, PutError};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::ops::Add;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
use tracing::{info, trace};

pub struct NodeState {
    handler: NodeHandler<ServerSignals>,
    listener: NodeListener<ServerSignals>,
    config: NodeConfig,
}

pub struct NodeConfig {
    id: Vec<u8>,
    /// The node's own address.
    self_addr: SocketAddr,
    /// Maps hashed key to file name.
    saved_files: HashMap<String, String>,
    ///List of successors node
    finger_table: Vec<SocketAddr>,
    predecessor: Option<SocketAddr>,
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

        let config = NodeConfig {
            id,
            self_addr,
            saved_files: Default::default(),
            finger_table: vec![],
            predecessor: None,
            gossip_interval: Default::default(),
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
                ServerSignals::ForwardPut(endpoint, file, addr) => {
                    trace!("Forwarding put");
                    forward_message(
                        &handler,
                        endpoint,
                        Message::ChordMessage(ChordMessage::ForwardedPut(addr, file)),
                    );
                }
                ServerSignals::ForwardGet(endpoint, addr, key) => {
                    trace!("Forwarding put");
                    forward_message(
                        &handler,
                        endpoint,
                        Message::ChordMessage(ChordMessage::ForwardedGet(addr, key)),
                    );
                }
            },
        });
    }
}

fn forward_message(handler: &NodeHandler<ServerSignals>, endpoint: Endpoint, message: impl Serialize) {
    let output_data = bincode::serialize(&message).unwrap();
    trace!("{}", endpoint.addr());
    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response");
        sleep(Duration::from_millis(1));
    }
}

///function to save the hashmap of key-file name
fn save_to_folder(saved_files: &HashMap<String, String>, port: u16) -> io::Result<()> {
    let folder_name = "server/".to_string() + port.to_string().as_str();
    let file_name = "saved_files.txt";

    // Create folder "x" if it doesn't exist
    if !Path::new(&folder_name).exists() {
        fs::create_dir(&folder_name)?;
    }

    // Create the file inside the folder
    let file_path = folder_name + file_name;
    let mut file = File::create(&file_path)?;

    // Write HashMap content to the file
    for (key, value) in saved_files {
        writeln!(file, "{}:{}", key, value)?;
    }

    println!("Saved HashMap to {}", file_path);
    Ok(())
}

fn load_from_folder(port: u16) -> io::Result<HashMap<String, String>> {
    let file_path = "server".to_string() + port.to_string().as_str() + "/saved_files.txt";
    let mut saved_files = HashMap::new();


    let file = File::open(&file_path)?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Some((key, value)) = line.split_once(':') {
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
        ChordMessage::AddPredecessor(addr) => config.predecessor = Some(addr),
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

fn handle_forwarded_get(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, addr: String, key: String) {
    let (endpoint, _) = handler.network().connect(Transport::Ws, &addr).unwrap();
    let message = get_from_key(handler, config, addr, key);
    forward_message(handler, endpoint, message);
}

fn get_from_key(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, addr: String, key: String) -> ServerToUserMessage {
    match handle_user_get(handler, config, addr, key.clone()) {
        Ok(file) => {
            ServerToUserMessage::RequestedFile(file)
        }
        Err(e) => {
            match e {
                GetError::ForwardingRequest(addr) => {
                    ServerToUserMessage::ForwarderTo(addr)
                }
                GetError::ErrorRetrievingFile => {
                    ServerToUserMessage::InternalServerError
                }
                GetError::NotFound => {
                    ServerToUserMessage::FileNotFound(key)
                }
                GetError::HexConversion => {
                    ServerToUserMessage::HexConversionNotValid(key)
                }
            }
        }
    }
}

fn handle_forwarded_put(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, addr: String, file: common::File) {
    match handle_user_put(handler, file, config, addr.clone()) {
        Ok(saved_key) => {
            trace!("Here something is broken---------------------");
            let (ep, _) = handler.network().connect(Transport::Ws, addr).unwrap();
            handler.signals().send(ServerSignals::SendMessageToUser(
                ep,
                ServerToUserMessage::SavedKey(saved_key),
            ));
        }
        Err(e) => match e {
            PutError::ForwardingRequest(forwarding_address) => {
                let (ep, _) = handler.network().connect(Transport::Ws, addr).unwrap();
                handler.signals().send(ServerSignals::SendMessageToUser(
                    ep,
                    ServerToUserMessage::ForwarderTo(forwarding_address),
                ));
            }
            PutError::ErrorStoringFile => {}
        },
    }
}

fn handle_user_message(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: UserMessage,
) {
    match message {
        UserMessage::Put(file, user_addr) => {
            trace!("Received file");
            match handle_user_put(handler, file, config, user_addr) {
                Ok(saved_key) => {
                    handler.network().send(
                        endpoint,
                        &bincode::serialize(&ServerToUserMessage::SavedKey(saved_key)).unwrap(),
                    );
                }
                Err(error) => match error {
                    PutError::ForwardingRequest(address) => {
                        handler.network().send(
                            endpoint,
                            &bincode::serialize(&ServerToUserMessage::ForwarderTo(address)).unwrap(),
                        );
                    }
                    PutError::ErrorStoringFile => {
                        trace!("problem occurred while saving file");
                    }
                },
            }
        }
        UserMessage::Get(key, user_addr) => {
            let send_message = get_from_key(handler, config, user_addr, key);
            handler.network().send(endpoint, &bincode::serialize(&send_message).unwrap());
        }
    }
}

fn handle_join(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, endpoint: Endpoint, addr: SocketAddr) {
    trace!("entering join process");
    let node_id = Sha256::digest(addr.to_string().as_bytes()).to_vec();

    if config.finger_table.is_empty() {
        insert_in_empty_table(handler, config, &endpoint, &addr);
        trace!("Node added to empty table");
        return;
    }

    let predecessor = Sha256::digest(config.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

    if node_id < config.id && node_id >= predecessor {
        trace!("Inserting between predecessor and self");
        insert_between_self_and_predecessor(handler, config, &endpoint, &addr);
        return;
    }

    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if node_id > config.id && (config.id > successor || node_id <= successor) {
        trace!("Inserting between self and successor");
        insert_between_self_and_successor(handler, config, &endpoint, &addr);
        return;
    }

    trace!("Starting forwarding process");

    forward_request(handler, config, &node_id, &endpoint);
}
fn handle_user_put(
    handler: &NodeHandler<ServerSignals>,
    file: common::File,
    config: &mut NodeConfig,
    addr: String,
) -> Result<String, PutError> {
    let digested_file_name = Sha256::digest(file.name.as_bytes()).to_vec();
    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if (digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
        || config.id > successor && digested_file_name < successor
        || config.finger_table.is_empty()
    {
        return save_in_server(file, config.self_addr.port(), config)
            .map_or(Err(PutError::ErrorStoringFile), Ok);
    }

    let forwarding_index = binary_search(config, &digested_file_name);

    let (forwarding_endpoint, _) = handler
        .network()
        .connect(Transport::Ws, config.finger_table[forwarding_index])
        .unwrap();

    handler
        .signals()
        .send(ServerSignals::ForwardPut(forwarding_endpoint, file, addr));

    Err(PutError::ForwardingRequest(
        config.finger_table[forwarding_index].to_string(),
    ))
}

fn save_in_server(file: common::File, port: u16, config: &mut NodeConfig) -> io::Result<String> {
    let common::File { name, buffer: data } = file;

    let digested_hex_file_name = hex::encode(Sha256::digest(name.as_bytes().to_vec()));

    trace!("hashed_file_name: {digested_hex_file_name}");

    let destination = &("server/"
        .to_string()
        .add(&port.to_string())
        .add("/")
        .add(&digested_hex_file_name));
    let path = Path::new(destination);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(destination)?;
    file.write_all(&data)?;
    config.saved_files.insert(digested_hex_file_name.clone(), name);

    trace!("File stored successfully");
    Ok(digested_hex_file_name)
}

fn handle_user_get(handler: &NodeHandler<ServerSignals>, config: &NodeConfig, key: String, addr: String) -> Result<common::File, GetError> {
    trace!("Handling user get");

    if let Ok(digested_file_name) = hex::decode(key.clone()) {
        let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

        if (digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
            || config.id > successor && digested_file_name < successor
            || config.finger_table.is_empty()
        {
            let file_name = config.saved_files.get(&key);

            if file_name.is_none() {
                trace!("No such a file");
                return Err(GetError::NotFound);
            }

            let file_path = "server/"
                .to_string()
                .add(&config.self_addr.port().to_string())
                .add("/")
                .add(&key);

            let file = File::open(file_path);

            let mut buffer = Vec::new();

            let _ = file.unwrap().read_to_end(&mut buffer); //todo check that works fine

            let file = common::File {
                name: file_name.unwrap().to_string(),
                buffer,
            };


            trace!("returning {}", file_name.unwrap());
            return Ok(file);
        }
        let forwarding_index = binary_search(config, &digested_file_name);

        let (forwarding_endpoint, _) = handler
            .network()
            .connect(Transport::Ws, config.finger_table[forwarding_index])
            .unwrap();
        handler
            .signals()
            .send(ServerSignals::ForwardGet(forwarding_endpoint, key, addr));

        return Err(GetError::ForwardingRequest(
            config.finger_table[forwarding_index].to_string(),
        ));
    }

    Err(GetError::HexConversion)
}

fn insert_in_empty_table(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    config.predecessor = Some(*addr);
    config.finger_table.push(*addr);

    let message = Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);

    let message = Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);
    trace!("join successfully");
}

fn insert_between_self_and_predecessor(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    let add_successor_message = Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr));
    let serialized = bincode::serialize(&add_successor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    let add_predecessor_message = Message::ChordMessage(ChordMessage::AddPredecessor(config.predecessor.unwrap()));
    let serialized = bincode::serialize(&add_predecessor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    config.predecessor = Some(*addr);
    trace!("join successfully");
}

fn insert_between_self_and_successor(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    //todo save in cache the successor 4 security reason
    let add_predecessor_message = Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr));
    let serialized = bincode::serialize(&add_predecessor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    let add_successor_message = Message::ChordMessage(ChordMessage::AddSuccessor(config.finger_table[0]));
    let serialized = bincode::serialize(&add_successor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    config.finger_table.insert(0, *addr);
    trace!("join successfully");
}

fn forward_request(handler: &NodeHandler<ServerSignals>, config: &NodeConfig, node_id: &Vec<u8>, endpoint: &Endpoint) {
    let forward_position = binary_search(config, node_id);
    let message = Message::ChordMessage(ChordMessage::ForwardedJoin(
        config.finger_table[forward_position].to_string(),
    ));
    let serialized = bincode::serialize(&message).unwrap();

    while handler.network().send(*endpoint, &serialized) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response...");
        sleep(Duration::from_millis(1000));
    }
}
fn binary_search(config: &NodeConfig, digested_vector: &Vec<u8>) -> usize {
    let mut s = 0;
    let mut e = config.finger_table.len();
    while s < e {
        let mid = (s + e) / 2;
        let mid_id = Sha256::digest(config.finger_table[mid].to_string().as_bytes()).to_vec();

        if mid_id > *digested_vector {
            e = mid;
        } else {
            s = mid + 1;
        }
    }

    if e == config.finger_table.len() {
        e -= 1;
    }

    e

    //todo Check if it's closer this one or the one that is predecessor
}
