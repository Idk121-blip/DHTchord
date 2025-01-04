mod join_handler;
mod user_message_handler;

use crate::common;
use crate::common::ChordMessage::{self};
use crate::common::{binary_search, get_endpoint, Message, ServerSignals, SERVER_FOLDER};
use crate::node_state::join_handler::handle_join;
use crate::node_state::user_message_handler::get_handler::{get_file_bytes, handle_forwarded_get};
use crate::node_state::user_message_handler::handle_user_message;
use crate::node_state::user_message_handler::put_handler::{handle_forwarded_put, save_in_server};
use message_io::network::{Endpoint, NetEvent, SendStatus, ToRemoteAddr, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::ops::Mul;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
use tracing::{error, info, trace};

//TODO TENERE IN ORDINE IL VETTORE CON GLI INDIRIZZI (FINGTAB)

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

    pub fn connect_and_run(self, socket_addr: SocketAddr) {
        let Self {
            handler,
            listener,
            mut config,
        } = self;
        let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));

        let serialized = bincode::serialize(&message).unwrap();

        let (endpoint, _) = handler.network().connect_sync(Transport::Ws, socket_addr).unwrap();

        config.finger_table_map.insert(socket_addr, endpoint);

        while handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
            trace!("Waiting for response...");
        }

        Self {
            handler,
            listener,
            config,
        }.run();
    }

    pub fn run(self) {
        let Self {
            handler,
            listener,
            mut config,
        } = self;

        info!("start");

        handler
            .signals()
            .send_with_timer(ServerSignals::Stabilization(), config.gossip_interval);

        //todo create a function to wrap this up
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
                    //trace!("Forwarding internal message");

                    let output_data = bincode::serialize(&message).unwrap();

                    if handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                        trace!(" Waiting for response {}", endpoint);
                        handler.signals().send(ServerSignals::ForwardMessage(endpoint, message))
                    }

                    //forward_message(&handler, endpoint, message, config.self_addr);
                }
                ServerSignals::SendMessageToUser(endpoint, message) => {
                    trace!("Forwarding message to user");
                    forward_message(&handler, endpoint, message, config.self_addr);
                }
                ServerSignals::Stabilization() => {
                    //TODO create a function to wrap this
                    trace!("Stabilization");
                    if config.gossip_interval.lt(&MAXIMUM_DURATION) {
                        let new_interval = config.gossip_interval.mul(2);
                        config.gossip_interval = new_interval;
                    }


                    let searching = binary_add(config.id.clone(), 0, ID_BYTES).unwrap();
                    let mut forwarding_index = binary_search(&config, &searching);

                    let socket_address = config.finger_table[forwarding_index];

                    let mut endpoint = get_endpoint(&handler, &mut config, socket_address);


                    handler.signals().send(ServerSignals::ForwardMessage(
                        endpoint,
                        Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
                    ));

                    for i in 0..FINGER_TABLE_SIZE {
                        let searching = binary_add(config.id.clone(), i, ID_BYTES).unwrap();
                        let new_forwarding_index = binary_search(&config, &searching);

                        let socket_address = config.finger_table[new_forwarding_index];

                        endpoint = get_endpoint(&handler, &mut config, socket_address);

                        handler.signals().send(ServerSignals::ForwardMessage(
                            endpoint,
                            Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
                        ));
                    }

                    //trace!("{:?}", config.finger_table);
                    handler
                        .signals()
                        .send_with_timer(ServerSignals::Stabilization(), config.gossip_interval);
                }
            },
        });
    }
}


fn should_be_in_finger_table(vec1: &[u8], vec2: &[u8]) -> bool {
    if vec1.len() != vec2.len() {
        return false;
    }

    for (&byte1, &byte2) in vec1.iter().zip(vec2.iter()) {
        let diff = (byte2 as i16 - byte1 as i16 + 256) % 256;
        if diff == 0 || (diff & (diff - 1)) != 0 {
            return false; // Difference must be a power of 2
        }
    }

    true
}

fn saved_file_folder_exist(port: u16) -> bool {
    let folder_name = SERVER_FOLDER.to_string() + port.to_string().as_str() + "/" + SAVED_FILES;
    Path::new(&folder_name).exists()
}

fn forward_message(handler: &NodeHandler<ServerSignals>, endpoint: Endpoint, message: impl Serialize, addr: SocketAddr) {
    let output_data = bincode::serialize(&message).unwrap();

    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
        trace!("{} Waiting for response {}",  addr, endpoint);
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
        ChordMessage::AddPredecessor(predecessor) => {
            config.predecessor = Some(predecessor);

            let forwarding_endpoint = get_endpoint(handler, config, predecessor);

            if forwarding_endpoint.addr() != endpoint.addr() {
                handler.signals().send(ServerSignals::ForwardMessage(
                    forwarding_endpoint,
                    Message::ChordMessage(ChordMessage::NotifyPredecessor(config.self_addr)),
                ));
            }
        }
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
        ChordMessage::AddSuccessor(successor) => {
            //trace!("Add successor {endpoint}, {mex} {}", self.config.self_addr);
            config.finger_table.insert(0, successor);
            move_files(handler, config, successor, &endpoint);

            let forwarding_endpoint = get_endpoint(handler, config, successor);

            if forwarding_endpoint.addr() == endpoint.addr() {
                trace!("{}, {:?}", config.self_addr, config.finger_table);
                return;
            }

            // handler.signals().send(ServerSignals::ForwardMessage(
            //     forwarding_endpoint,
            //     Message::ChordMessage(ChordMessage::NotifySuccessor(config.self_addr)),
            // ));

            trace!("{}, {:?}", config.self_addr, config.finger_table);
        }
        ChordMessage::ForwardedJoin(addr) => {
            trace!("Forwarded join, joining {addr}");

            let new_endpoint = get_endpoint(handler, config, addr);
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
        ChordMessage::MoveFile(file) => {
            let _ = save_in_server(file, config.self_addr.port(), config);
        }

        ChordMessage::NotifySuccessor(predecessor) => {
            if config.predecessor.unwrap() == predecessor {
                return;
            }
            config.predecessor = Some(predecessor);
        }
        ChordMessage::NotifyPredecessor(successor) => {
            if config.finger_table[0] == successor {
                return;
            }
            config.finger_table.insert(0, successor);

            move_files(handler, config, successor, &endpoint);
            //todo remove the first one if it's not n+2^i id
        }
        ChordMessage::Find(wanted_id, searching_address) => {
            if wanted_id == config.id {
                let searching_endpoint = get_endpoint(handler, config, searching_address);
                handler.signals().send(ServerSignals::ForwardMessage(
                    searching_endpoint,
                    Message::ChordMessage(ChordMessage::NotifyPresence(config.self_addr)),
                ));
                return;
            }

            let index = binary_search(config, &wanted_id);

            let digested_address = Sha256::digest(config.finger_table[index].to_string().as_bytes()).to_vec();

            if digested_address == wanted_id {
                //iterative way
                let searching_endpoint = get_endpoint(handler, config, searching_address);
                handler.signals().send(ServerSignals::ForwardMessage(
                    searching_endpoint,
                    Message::ChordMessage(ChordMessage::NotifyPresence(config.finger_table[index])),
                ));
                return;
            }

            let digested_ip_address_request = Sha256::digest(searching_address.to_string().as_bytes()).to_vec();

            //3 cases 1) if we are returning to the "starting" point, 2) if the node doesn't exist
            // 3) if we are restarting the circle (9->0, and we are looking for 10)
            if (config.id > wanted_id
                && (digested_ip_address_request < wanted_id
                || wanted_id < digested_address
                || digested_address < config.id)) || digested_ip_address_request == digested_address
            {
                //not found no need to send a response since it will increase the traffic
                return;
            }

            let forwarding_address = config.finger_table[index];


            let forwarding_endpoint = get_endpoint(handler, config, forwarding_address);

            handler.signals().send(ServerSignals::ForwardMessage(
                forwarding_endpoint,
                Message::ChordMessage(ChordMessage::Find(wanted_id, searching_address)),
            ));
        }
        ChordMessage::NotifyPresence(addr) => {
            trace!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
            let digested_address = Sha256::digest(addr.to_string().as_bytes()).to_vec();
            let index = binary_search(config, &digested_address);
            trace!("{:?}\n {:?}",digested_address, config.id);
            if config.finger_table[index] == addr {
                return;
            }
            config.finger_table.insert(index, addr);
            trace!("Node added to finger table ");
        }
    }
}

fn move_files(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    new_node_addr: SocketAddr,
    endpoint: &Endpoint,
) {
    let digested_addr = Sha256::digest(new_node_addr.to_string().as_bytes()).to_vec();

    let forward_endpoint = get_endpoint(handler, config, new_node_addr);

    trace!("{} {}", endpoint.addr(), forward_endpoint.addr());

    for (key, file_name) in &config.saved_files {
        let digested_key = hex::decode(key).unwrap();
        if digested_key > digested_addr {
            //todo move the file that is saved

            let file_path = SERVER_FOLDER.to_string() + config.self_addr.port().to_string().as_str() + "/" + key;
            let buffer = get_file_bytes(file_path.clone());
            handler.signals().send(ServerSignals::ForwardMessage(
                forward_endpoint,
                Message::ChordMessage(ChordMessage::MoveFile(common::File {
                    name: file_name.to_string(),
                    buffer,
                })),
            ));
            fs::remove_file(file_path).unwrap()
        }
    }
}

fn binary_add(mut vec: Vec<u8>, index: usize, bytes: usize) -> Result<Vec<u8>, ()> {
    let mut byte = bytes - 1;
    let power = (index % 8) as u8;
    if (byte as isize - (index / 8) as isize) < 0 {
        return Err(());
    }

    byte -= index / 8;
    let mut specific_byte = vec[byte] as u16;

    let mut x = 1;
    x <<= power;

    specific_byte += x;
    let mut carry;
    if specific_byte > 255 {
        while specific_byte > 255 {
            carry = specific_byte / 255;

            vec[byte] = (specific_byte % 255) as u8;

            if byte as isize - 1 < 0 {
                break;
            }
            byte -= 1;
            specific_byte = carry + vec[byte] as u16;
        }
    } else {
        vec[byte] = (specific_byte % 255) as u8;
    }
    Ok(vec)
}
