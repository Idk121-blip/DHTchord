use crate::common::ChordMessage::{self};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::thread::sleep;
use std::time::Duration;

pub struct NodeState {
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
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
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let (node_handler, node_listener) = node::split();

        let listen_addr = SocketAddr::new(ip, port);

        node_handler
            .network()
            .listen(Transport::FramedTcp, listen_addr)
            .unwrap();

        println!("Discovery server running at {}", listen_addr);

        let id = Sha256::digest(listen_addr.to_string().as_bytes()).to_vec();

        Self {
            node_handler,
            id,
            node_listener: Some(node_listener),
            self_addr: SocketAddr::new(ip, port),
            known_peers: Default::default(),
            finger_table: vec![],
            predecessor: None,
            gossip_interval: Default::default(),
            sha: Sha256::new(),
        }
    }

    pub fn run(mut self) {
        if self.self_addr.port() == 9000 {
            let mex2 = ChordMessage::Join(self.self_addr);
            match bincode::serialize(&mex2) {
                Ok(output_data) => {
                    let (endpoint, _) = self
                        .node_handler
                        .network()
                        .connect(Transport::FramedTcp, "127.0.0.1:8911")
                        .unwrap();

                    while self.node_handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                        println!("Waiting for response...");
                    } //todo work on this for a better mechanism
                }
                Err(x) => {
                    println!("{:?}", x);
                }
            }
        }

        if self.self_addr.port() == 8910 {
            let mex2 = ChordMessage::Join(self.self_addr);
            match bincode::serialize(&mex2) {
                Ok(output_data) => {
                    let (endpoint, _) = self
                        .node_handler
                        .network()
                        .connect(Transport::FramedTcp, "127.0.0.1:8911")
                        .unwrap();

                    while self.node_handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                        println!("Waiting for response...");
                    } //todo work on this for a better mechanism
                }
                Err(x) => {
                    println!("{:?}", x);
                }
            }
        }
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event.network() {
            NetEvent::Message(endpoint, input_data) => {
                //
                match bincode::deserialize(input_data) {
                    Ok(message) => {
                        self.handle_message(endpoint, message);
                    }
                    Err(x) => println!("{:?}", x),
                }
            }
            NetEvent::Connected(endpoint, _result) => {
                println!("{}: request from ip: {endpoint} connected: {_result}", self.self_addr);
                println!("{:?}", self.node_handler.is_running());
                println!("{:?}", self.node_handler.network().is_ready(endpoint.resource_id()));
                // self.node_handler.network().connect(Transport::FramedTcp, endpoint.addr());
            }
            NetEvent::Accepted(_, _) => {}
            NetEvent::Disconnected(x) => {
                self.node_handler.network().remove(x.resource_id());
            }
        });
    }

    fn handle_message(&mut self, endpoint: Endpoint, message: ChordMessage) {
        match message {
            ChordMessage::RegisterServer(_x1, x2) => {
                //todo remove (credo, i have to check)
                if self.known_peers.contains(&x2) {
                    println!("Server is already in");
                }
            }
            ChordMessage::ScanningFor(_, _) => {}
            ChordMessage::ServerAdded(_, _) => {}
            ChordMessage::SendStringMessage(mex, addr) => {
                let mex2 = ChordMessage::Message(mex);
                println!("Send message from {}, {}", addr, self.self_addr);
                let output_data = bincode::serialize(&mex2).unwrap();

                //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
                println!("{endpoint}");
                //sleep(Duration::from_secs(3));
                self.node_handler.network().send(endpoint, &output_data);
            }
            ChordMessage::Join(socket_addr) => {
                println!(
                    "{}: request from endpoint: {endpoint} ip: {socket_addr}",
                    self.self_addr
                );
                self.handle_join(endpoint, socket_addr);
                //Find the closest to that position
            }
            ChordMessage::Message(mex) => {
                println!("Message received from other peer");
                println!("My ip {:?}", self.self_addr);
                println!("{mex}")
            }
            ChordMessage::AddSuccessor(address) => {
                //println!("Add successor {endpoint}, {mex} {}", self.self_addr);
                self.finger_table.insert(0, address);

                println!("{}, {:?}", self.self_addr, self.finger_table);
            }
            ChordMessage::AddPredecessor(address) => self.predecessor = Some(address),
            ChordMessage::ForwardedJoin(socket_addr) => {
                println!("Oh shit. here we go again");

                let join_message = ChordMessage::Join(self.self_addr);
                let output_data = bincode::serialize(&join_message).unwrap();

                self.node_handler.network().remove(endpoint.resource_id());

                let (new_endpoint, _) = self
                    .node_handler
                    .network()
                    .connect(Transport::FramedTcp, socket_addr)
                    .unwrap();

                println!("{:?}", self.node_handler.network().is_ready(new_endpoint.resource_id()));

                while self.node_handler.network().send(new_endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                    println!("Waiting for response...");
                    sleep(Duration::from_secs(1));
                }
            }

            _ => {}
        }
    }

    fn handle_join(&mut self, endpoint: Endpoint, socket_addr: SocketAddr) {
        println!("entering join process");

        let node_id = Sha256::digest(socket_addr.to_string().as_bytes()).to_vec();

        if self.has_empty_table(&endpoint, &socket_addr) {
            self.node_handler.network().remove(endpoint.resource_id());
            println!("Node added to empty table");
            return;
        }

        let successor = Sha256::digest(self.finger_table[0].to_string().as_bytes()).to_vec();

        let predecessor = Sha256::digest(self.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

        if self.insert_near_self(&endpoint, &socket_addr, &node_id, successor, predecessor) {
            self.node_handler.network().remove(endpoint.resource_id());
            return;
        }

        println!("Good so far");

        self.binary_search(&node_id, &endpoint);

        self.node_handler.network().remove(endpoint.resource_id());
    }

    fn has_empty_table(&mut self, endpoint: &Endpoint, socket_addr: &SocketAddr) -> bool {
        if self.finger_table.is_empty() {
            println!("{}: request from ip: {endpoint} joining: {socket_addr}", self.self_addr);
            self.predecessor = Some(*socket_addr);
            self.finger_table.push(*socket_addr);

            let message = bincode::serialize(&ChordMessage::AddSuccessor(self.self_addr)).unwrap();
            self.node_handler.network().send(*endpoint, &message);
            let message = bincode::serialize(&ChordMessage::AddPredecessor(self.self_addr)).unwrap();
            self.node_handler.network().send(*endpoint, &message);
            println!("{:?}", self.finger_table);
            println!("join successfully");
            return true;
        }
        false
    }

    fn insert_near_self(
        &mut self,
        endpoint: &Endpoint,
        socket_addr: &SocketAddr,
        node_id: &Vec<u8>,
        predecessor: Vec<u8>,
        successor: Vec<u8>,
    ) -> bool {
        self.insert_between(endpoint, socket_addr, node_id, predecessor, true)
            || self.insert_between(endpoint, socket_addr, node_id, successor, false)
    }

    fn insert_between(
        &mut self,
        endpoint: &Endpoint,
        socket_addr: &SocketAddr,
        node_id: &Vec<u8>,
        node_near_self: Vec<u8>,
        is_predecessor: bool,
    ) -> bool {
        //todo save in cache the successor 4 security reason

        if (is_predecessor && *node_id < self.id && *node_id >= node_near_self)
            || (!is_predecessor && *node_id > self.id && *node_id <= node_near_self)
        {
            if is_predecessor {
                println!("{}: Inserting between predecessor and self", self.self_addr);
            } else {
                println!("{}: Inserting between self and successor", self.self_addr);
            };

            let first_message = if is_predecessor {
                ChordMessage::AddSuccessor(self.self_addr)
            } else {
                ChordMessage::AddPredecessor(self.self_addr)
            };
            let output_data = bincode::serialize(&first_message).unwrap();
            self.node_handler.network().send(*endpoint, &output_data);

            let second_message = if is_predecessor {
                ChordMessage::AddPredecessor(self.predecessor.unwrap())
            } else {
                ChordMessage::AddSuccessor(self.finger_table[0])
            };
            let output_data = bincode::serialize(&second_message).unwrap();
            self.node_handler.network().send(*endpoint, &output_data);

            if is_predecessor {
                self.predecessor = Some(*socket_addr);
            } else {
                self.finger_table.insert(0, *socket_addr);
            }

            println!("{:?}", self.finger_table);
            println!("join successfully");

            return true;
        }
        false
    }

    fn binary_search(&self, node_id: &Vec<u8>, endpoint: &Endpoint) {
        let mut s = 0;
        let mut e = self.finger_table.len();
        while s < e {
            let mid = (s + e) / 2;
            let mid_id = Sha256::digest(self.finger_table[mid].to_string().as_bytes()).to_vec();

            if mid_id > *node_id {
                e = mid;
            } else {
                s = mid + 1;
            }
        }

        println!("{}, {s}: search successfully", e);
        println!("{}", self.finger_table[e]);

        println!("{}, {}", self.finger_table[e], endpoint);

        let message = ChordMessage::ForwardedJoin(self.finger_table[e]);
        let output_data = bincode::serialize(&message).unwrap();

        while self.node_handler.network().send(*endpoint, &output_data) == SendStatus::ResourceNotAvailable {
            println!("Waiting for response...");
            sleep(Duration::from_millis(1000));
        }

        // self.node_handler.network().remove(endpoint2.resource_id());
        // self.node_handler.network().remove(endpoint.resource_id());

        //todo Check if it's closer this one or the one that is predecessor
        // NB: it won't be unless mid = e at the end (CREDO)
    }
}
