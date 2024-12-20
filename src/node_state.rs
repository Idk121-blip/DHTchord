use crate::common::ChordMessage::{self};
use crate::common::{Message, Signals, UserMessage};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::ops::Add;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
use tracing::{info, trace};

pub struct NodeState {
    handler: NodeHandler<Signals>,
    listener: NodeListener<Signals>,
    config: NodeConfig,
}

pub struct NodeConfig {
    id: Vec<u8>,
    /// The node's own address.
    self_addr: SocketAddr,
    /// Known peers in the network.
    known_peers: HashSet<SocketAddr>,
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
            known_peers: Default::default(),
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
                Signals::ForwardMessage(endpoint, message) => {
                    trace!("Starting Forwarded Join");
                    let output_data = bincode::serialize(&message).unwrap();
                    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                        trace!("Waiting for response");
                    }
                }
                Signals::ForwardPut(endpoint, file) => {
                    trace!("Forwarding put");
                    let output_data = bincode::serialize(&Message::UserMessage(UserMessage::Put(file))).unwrap();
                    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                        trace!("Waiting for response");
                    }
                }
            },
        });
    }
}
fn handle_server_message(
    handler: &NodeHandler<Signals>,
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
            handler.signals().send(Signals::ForwardMessage(new_endpoint, message));
        }
        ChordMessage::ForwardedPut(addr, file) => {
            trace!("Forwarded put");
            handle_user_put(handler, file, config).unwrap();
            //todo connect to user and send message with all is needed
        }
    }
}

fn handle_user_message(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: UserMessage,
) {
    match message {
        UserMessage::Put(file) => {
            //forwardare il put per ora salvataggio in cartella qui
            trace!("Received file");
            let _ = handle_user_put(handler, file, config);
            //todo send message to client
        }
        UserMessage::Get(name, extension) => {}
    }
}

fn handle_join(handler: &NodeHandler<Signals>, config: &mut NodeConfig, endpoint: Endpoint, addr: SocketAddr) {
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
fn handle_user_put(handler: &NodeHandler<Signals>, file: crate::common::File, config: &NodeConfig) -> io::Result<()> {
    let digested_file_name = Sha256::digest(file.name.as_bytes()).to_vec();
    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if (digested_file_name > config.id && digested_file_name < successor) || config.finger_table.is_empty() {
        return save_in_server(file, config.self_addr.port() as usize);
    }

    let forwarding_index = binary_search(config, &digested_file_name);

    let (forwarding_endpoint, _) = handler
        .network()
        .connect(Transport::Ws, config.finger_table[forwarding_index])?;

    handler.signals().send(Signals::ForwardPut(forwarding_endpoint, file));

    Ok(())
}

fn save_in_server(file: crate::common::File, port: usize) -> io::Result<()> {
    let crate::common::File { name, extension, data } = file;
    let destination = &("server/"
        .to_string()
        .add(&port.to_string())
        .add("/")
        .add(name.as_str())
        .add(extension.as_str()));
    let path = Path::new(destination);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(destination)?;
    file.write_all(&data)?;
    trace!("File stored successfully");
    Ok(())
}

fn insert_in_empty_table(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    trace!("{}: request from ip: {endpoint} joining: {addr}", config.self_addr);
    config.predecessor = Some(*addr);
    config.finger_table.push(*addr);

    let message = Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);

    let message = Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);
    trace!("{:?}", config.finger_table);
    trace!("join successfully");
}

fn insert_between_self_and_predecessor(
    handler: &NodeHandler<Signals>,
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
    trace!("{:?}", config.finger_table);
    trace!("join successfully");
}

fn insert_between_self_and_successor(
    handler: &NodeHandler<Signals>,
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
    trace!("{:?}", config.finger_table);
    trace!("join successfully");
}

fn forward_request(handler: &NodeHandler<Signals>, config: &NodeConfig, node_id: &Vec<u8>, endpoint: &Endpoint) {
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

    // node_handler.network().remove(endpoint2.resource_id());
    // node_handler.network().remove(endpoint.resource_id());

    //todo Check if it's closer this one or the one that is predecessor
    // NB: it won't be unless mid = e at the end (CREDO)
}
