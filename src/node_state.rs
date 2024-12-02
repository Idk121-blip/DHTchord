use crate::common::ChordMessage::{self};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::thread::sleep;
use std::time::Duration;
use tracing::{info, trace};

pub struct NodeState {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
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

        let socket = SocketAddr::new(ip, port);

        handler.network().listen(Transport::FramedTcp, socket)?;

        let id = Sha256::digest(socket.to_string().as_bytes()).to_vec();

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
            mut handler,
            listener,
            mut config,
        } = self;

        info!("start");
        if config.self_addr.port() == 9000 {
            let message = ChordMessage::Join(config.self_addr);
            let serialized = bincode::serialize(&message).unwrap();

            let (endpoint, _) = handler
                .network()
                .connect(Transport::FramedTcp, "127.0.0.1:8911")
                .unwrap();

            while handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
                trace!("Waiting for response...");
            } //todo work on this for a better mechanism
        }

        if config.self_addr.port() == 8910 {
            let message = ChordMessage::Join(config.self_addr);
            let serialized = bincode::serialize(&message).unwrap();

            let (endpoint, _) = handler
                .network()
                .connect(Transport::FramedTcp, "127.0.0.1:8911")
                .unwrap();

            while handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {
                trace!("Waiting for response...");
            } //todo work on this for a better mechanism
        }

        listener.for_each(move |event| match event.network() {
            NetEvent::Message(endpoint, input_data) => {
                //
                match bincode::deserialize(input_data) {
                    Ok(message) => {
                        handle_message(&mut handler, &mut config, endpoint, message);
                    }
                    Err(x) => trace!("{:?}", x),
                }
            }
            NetEvent::Connected(endpoint, _result) => {
                trace!("request from ip: {endpoint} connected: {_result}");
                trace!("{:?}", handler.is_running());
                trace!("{:?}", handler.network().is_ready(endpoint.resource_id()));
                // self.node_handler.network().connect(Transport::FramedTcp, endpoint.addr());
            }
            NetEvent::Accepted(_, _) => {}
            NetEvent::Disconnected(x) => {
                handler.network().remove(x.resource_id());
            }
        });
    }
}

fn has_empty_table(
    handler: &NodeHandler<()>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    socket_addr: &SocketAddr,
) -> bool {
    if config.finger_table.is_empty() {
        trace!(
            "{}: request from ip: {endpoint} joining: {socket_addr}",
            config.self_addr
        );
        config.predecessor = Some(*socket_addr);
        config.finger_table.push(*socket_addr);

        let message = bincode::serialize(&ChordMessage::AddSuccessor(config.self_addr)).unwrap();
        handler.network().send(*endpoint, &message);
        let message = bincode::serialize(&ChordMessage::AddPredecessor(config.self_addr)).unwrap();
        handler.network().send(*endpoint, &message);
        trace!("{:?}", config.finger_table);
        trace!("join successfully");
        return true;
    }
    false
}

fn insert_near_self(
    handler: &NodeHandler<()>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    socket_addr: &SocketAddr,
    node_id: &Vec<u8>,
    predecessor: Vec<u8>,
    successor: Vec<u8>,
) -> bool {
    insert_between(handler, config, endpoint, socket_addr, node_id, predecessor, true)
        || insert_between(handler, config, endpoint, socket_addr, node_id, successor, false)
}

fn insert_between(
    handler: &NodeHandler<()>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    socket_addr: &SocketAddr,
    node_id: &Vec<u8>,
    node_near_self: Vec<u8>,
    is_predecessor: bool,
) -> bool {
    //todo save in cache the successor 4 security reason

    if (is_predecessor && *node_id < config.id && *node_id >= node_near_self)
        || (!is_predecessor && *node_id > config.id && *node_id <= node_near_self)
    {
        if is_predecessor {
            trace!("{}: Inserting between predecessor and self", config.self_addr);
        } else {
            trace!("{}: Inserting between self and successor", config.self_addr);
        };

        let first_message = if is_predecessor {
            ChordMessage::AddSuccessor(config.self_addr)
        } else {
            ChordMessage::AddPredecessor(config.self_addr)
        };
        let output_data = bincode::serialize(&first_message).unwrap();
        handler.network().send(*endpoint, &output_data);

        let second_message = if is_predecessor {
            ChordMessage::AddPredecessor(config.predecessor.unwrap())
        } else {
            ChordMessage::AddSuccessor(config.finger_table[0])
        };
        let output_data = bincode::serialize(&second_message).unwrap();
        handler.network().send(*endpoint, &output_data);

        if is_predecessor {
            config.predecessor = Some(*socket_addr);
        } else {
            config.finger_table.insert(0, *socket_addr);
        }

        trace!("{:?}", config.finger_table);
        trace!("join successfully");

        return true;
    }
    false
}

fn binary_search(handler: &NodeHandler<()>, config: &NodeConfig, node_id: &Vec<u8>, endpoint: &Endpoint) {
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

    trace!("{}, {s}: search successfully", e);
    trace!("{}", config.finger_table[e]);

    trace!("{}, {}", config.finger_table[e], endpoint);

    let message = ChordMessage::ForwardedJoin(config.finger_table[e]);
    let output_data = bincode::serialize(&message).unwrap();

    while handler.network().send(*endpoint, &output_data) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response...");
        sleep(Duration::from_millis(1000));
    }

    // node_handler.network().remove(endpoint2.resource_id());
    // node_handler.network().remove(endpoint.resource_id());

    //todo Check if it's closer this one or the one that is predecessor
    // NB: it won't be unless mid = e at the end (CREDO)
}

fn handle_message(handler: &NodeHandler<()>, config: &mut NodeConfig, endpoint: Endpoint, message: ChordMessage) {
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
            let mex2 = ChordMessage::Message(mex);
            trace!("Send message from {}, {}", addr, config.self_addr);
            let output_data = bincode::serialize(&mex2).unwrap();

            //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
            trace!("{endpoint}");
            //sleep(Duration::from_secs(3));
            handler.network().send(endpoint, &output_data);
        }
        ChordMessage::Join(socket_addr) => {
            trace!("request from endpoint: {endpoint} ip: {socket_addr}",);
            handle_join(handler, config, endpoint, socket_addr);
            //Find the closest to that position
        }
        ChordMessage::Message(mex) => {
            trace!("Message received from other peer: {mex}");
        }
        ChordMessage::AddSuccessor(address) => {
            //trace!("Add successor {endpoint}, {mex} {}", self.config.self_addr);
            config.finger_table.insert(0, address);

            trace!("{}, {:?}", config.self_addr, config.finger_table);
        }
        ChordMessage::AddPredecessor(address) => config.predecessor = Some(address),
        ChordMessage::ForwardedJoin(socket_addr) => {
            trace!("Oh shit. here we go again");

            let join_message = ChordMessage::Join(config.self_addr);
            let output_data = bincode::serialize(&join_message).unwrap();

            handler.network().remove(endpoint.resource_id());

            let (new_endpoint, _) = handler.network().connect(Transport::FramedTcp, socket_addr).unwrap();

            trace!("{:?}", handler.network().is_ready(new_endpoint.resource_id()));

            while handler.network().send(new_endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                trace!("Waiting for response...");
                sleep(Duration::from_secs(1));
            }
        }

        _ => {}
    }
}

fn handle_join(handler: &NodeHandler<()>, config: &mut NodeConfig, endpoint: Endpoint, socket_addr: SocketAddr) {
    trace!("entering join process");

    let node_id = Sha256::digest(socket_addr.to_string().as_bytes()).to_vec();

    if has_empty_table(handler, config, &endpoint, &socket_addr) {
        handler.network().remove(endpoint.resource_id());
        trace!("Node added to empty table");
        return;
    }

    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    let predecessor = Sha256::digest(config.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

    if insert_near_self(
        handler,
        config,
        &endpoint,
        &socket_addr,
        &node_id,
        successor,
        predecessor,
    ) {
        handler.network().remove(endpoint.resource_id());
        return;
    }

    trace!("Good so far");

    binary_search(handler, config, &node_id, &endpoint);

    handler.network().remove(endpoint.resource_id());
}
