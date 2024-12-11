use crate::common::ChordMessage::{self};
use crate::common::UserMessage::Put;
use crate::common::{Message, Signals, UserMessage};
use message_io::network::SendStatus::ResourceNotAvailable;
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use oneshot::Sender;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::thread::sleep;
use std::time::Duration;
use tracing::{info, trace};

pub struct NodeState {
    handler: NodeHandler<Signals>,
    listener: NodeListener<Signals>,
    config: NodeConfig,
}

pub struct User {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
}

impl User {
    pub fn new() -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let (id, listen_address) = handler.network().listen(Transport::Ws, "0.0.0.0:0")?;
        trace!("{listen_address}");
        Ok(Self { handler, listener })
    }

    pub fn put(self, server_address: String, sender: Sender<String>) {
        let (ep, _) = self
            .handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();
        let sender_option = Some(sender);
        self.handler
            .network()
            .send(ep, &bincode::serialize(&Message::UserMessage(Put(12))).unwrap());
        self.listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, _) => {
                trace!("response from server, killing myself");
                // processor.sender_option.unwrap().send("message received".to_string());
                self.handler.stop();
            }
            NetEvent::Disconnected(_) => {}
        });
    }
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

        let addr = SocketAddr::new(ip, port);

        handler.network().listen(Transport::Ws, addr)?;

        let id = Sha256::digest(addr.to_string().as_bytes()).to_vec();

        let config = NodeConfig {
            id,
            self_addr: SocketAddr::new(ip, port),
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

    pub fn run(self) {
        let Self {
            handler,
            listener,
            mut config,
        } = self;

        info!("start");
        if config.self_addr.port() == 7777 {
            let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));

            let serialized = bincode::serialize(&message).unwrap();

            let (endpoint, _) = handler.network().connect_sync(Transport::Ws, "127.0.0.1:8911").unwrap();

            while handler.network().send(endpoint, &serialized) == ResourceNotAvailable {
                trace!("Waiting for response...");
            } //todo work on this for a better mechanism
        }

        if config.self_addr.port() == 8910 {
            let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));
            let serialized = bincode::serialize(&message).unwrap();

            let (endpoint, _) = handler.network().connect_sync(Transport::Ws, "127.0.0.1:8911").unwrap();

            while handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
                trace!("Waiting for response...");
            } //todo work on this for a better mechanism
        }

        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => {
                match net_event {
                    NetEvent::Message(endpoint, serialized) => {
                        //
                        let message = bincode::deserialize(serialized).unwrap();

                        match message {
                            Message::UserMessage(user_message) => {
                                trace!("Received user message: {:?}", user_message);
                                handle_user_message(&handler, &mut config, endpoint, user_message);
                            }
                            Message::ChordMessage(server_message) => {
                                handle_server_message(&handler, &mut config, endpoint, server_message);
                            }
                        }
                    }
                    NetEvent::Connected(endpoint, result) => {
                        trace!("request from ip: {endpoint} connected: {result}");
                        // self.node_handler.network().connect(Transport::FramedTcp, endpoint.addr());
                    }
                    NetEvent::Accepted(endpoint, rid) => {
                        trace!("Communication accepted");
                    }
                    NetEvent::Disconnected(endpoint) => {}
                }
            }
            NodeEvent::Signal(signal) => match signal {
                Signals::ForwardedJoin(endpoint) => {
                    trace!("Starting Forwarded Join");
                    let output_data =
                        bincode::serialize(&Message::ChordMessage(ChordMessage::Join(config.self_addr))).unwrap();
                    while handler.network().send(endpoint, &output_data) == ResourceNotAvailable {
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
        ChordMessage::RegisterServer(_x1, x2) => {
            //todo remove (credo, i have to check)
            if config.known_peers.contains(&x2) {
                trace!("Server is already in");
            }
        }
        ChordMessage::ScanningFor(_, _) => {}
        ChordMessage::ServerAdded(_, _) => {}
        ChordMessage::SendStringMessage(mex, addr) => {
            let message = ChordMessage::Message(mex);
            trace!("Send message from {}, {}", addr, config.self_addr);
            let outpud_data = bincode::serialize(&message).unwrap();

            //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
            trace!("{endpoint}");
            //sleep(Duration::from_secs(3));
            handler.network().send(endpoint, &outpud_data);
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

            handler.signals().send(Signals::ForwardedJoin(new_endpoint));
        }

        _ => {}
    }
}

fn handle_user_message(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: UserMessage<isize>,
) {
}

fn has_empty_table(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) -> bool {
    if config.finger_table.is_empty() {
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
        return true;
    }
    false
}

fn insert_near_self(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
    node_id: &Vec<u8>,
    predecessor: &Vec<u8>,
    successor: &Vec<u8>,
) -> bool {
    insert_between(handler, config, endpoint, addr, node_id, predecessor, true)
        || insert_between(handler, config, endpoint, addr, node_id, successor, false)
}

fn insert_between(
    handler: &NodeHandler<Signals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
    node_id: &Vec<u8>,
    node_near_self: &Vec<u8>,
    is_predecessor: bool,
) -> bool {
    //todo save in cache the successor 4 security reason

    if (is_predecessor && *node_id < config.id && *node_id >= *node_near_self)
        || (!is_predecessor && (*node_id > config.id && (config.id > *node_near_self || *node_id <= *node_near_self)))
    {
        if is_predecessor {
            trace!("{}: Inserting between predecessor and self", config.self_addr);
        } else {
            trace!("{}: Inserting between self and successor", config.self_addr);
        };

        let first_message = if is_predecessor {
            Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr))
        } else {
            Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr))
        };
        let serialized = bincode::serialize(&first_message).unwrap();
        handler.network().send(*endpoint, &serialized);

        let second_message = if is_predecessor {
            Message::ChordMessage(ChordMessage::AddPredecessor(config.predecessor.unwrap()))
        } else {
            Message::ChordMessage(ChordMessage::AddSuccessor(config.finger_table[0]))
        };
        let serialized = bincode::serialize(&second_message).unwrap();
        handler.network().send(*endpoint, &serialized);

        if is_predecessor {
            config.predecessor = Some(*addr);
        } else {
            config.finger_table.insert(0, *addr);
        }

        trace!("{:?}", config.finger_table);
        trace!("join successfully");

        return true;
    }
    false
}

fn binary_search(handler: &NodeHandler<Signals>, config: &NodeConfig, node_id: &Vec<u8>, endpoint: &Endpoint) {
    let mut s = 0;
    let mut e = config.finger_table.len();
    while s < e {
        let mid = (s + e) / 2;
        let mid_id = Sha256::digest(config.finger_table[mid].to_string().as_bytes()).to_vec();

        if mid_id > *node_id {
            e = mid;
        } else {
            s = mid + 1;
        }
    }

    if e == config.finger_table.len() {
        e -= 1;
    }

    let message = Message::ChordMessage(ChordMessage::ForwardedJoin(config.finger_table[e].to_string()));
    let serialized = bincode::serialize(&message).unwrap();

    while handler.network().send(*endpoint, &serialized) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response...");
        sleep(Duration::from_millis(1000));
    }

    // node_handler.network().remove(endpoint2.resource_id());
    // node_handler.network().remove(endpoint.resource_id());

    //todo Check if it's closer this one or the one that is predecessor
    // NB: it won't be unless mid = e at the end (CREDO)
}

fn handle_join(handler: &NodeHandler<Signals>, config: &mut NodeConfig, endpoint: Endpoint, addr: SocketAddr) {
    trace!("entering join process");

    let node_id = Sha256::digest(addr.to_string().as_bytes()).to_vec();

    if has_empty_table(handler, config, &endpoint, &addr) {
        trace!("Node added to empty table");
        return;
    }

    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    let predecessor = Sha256::digest(config.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

    if insert_near_self(handler, config, &endpoint, &addr, &node_id, &successor, &predecessor) {
        trace!("Node added near self");
        return;
    }

    trace!("About to perform a binary search");

    binary_search(handler, config, &node_id, &endpoint);

    //handler.network().remove(endpoint.resource_id());
}
