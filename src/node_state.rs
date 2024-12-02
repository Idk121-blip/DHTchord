use crate::common::Message::{self};
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
    pub fn new(ip_addr: IpAddr, port: u16) -> Self {
        let (node_handler, node_listener) = node::split();

        let listen_addr = SocketAddr::new(ip_addr, port);

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
            self_addr: SocketAddr::new(ip_addr, port),
            known_peers: Default::default(),
            finger_table: vec![],
            predecessor: None,
            gossip_interval: Default::default(),
            sha: Sha256::new(),
        }
    }

    pub fn run(mut self) {
        if self.self_addr.port() == 9000 {
            let mex2 = Message::Join(self.self_addr.clone());
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
            let mex2 = Message::Join(self.self_addr.clone());
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
                self.binary_handler(endpoint, input_data);
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

    fn binary_handler(&mut self, endpoint: Endpoint, input_data: &[u8]) {
        match bincode::deserialize(&input_data) {
            Ok(message) => {
                self.message_handler(endpoint, message);
            }
            Err(x) => println!("{:?}", x),
        }
    }

    fn message_handler(&mut self, endpoint: Endpoint, message: Message) {
        match message {
            Message::RegisterServer(_x1, x2) => {
                //todo remove (credo, i have to check)
                if self.known_peers.contains(&x2) {
                    println!("Server is already in");
                    return;
                }
            }
            Message::ScanningFor(_, _) => {}
            Message::ServerAdded(_, _) => {}
            Message::SendStringMessage(mex, addr) => {
                let mex2 = Message::Message(mex);
                println!("Send message from {}, {}", addr, self.self_addr);
                let output_data = bincode::serialize(&mex2).unwrap();

                //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
                println!("{endpoint}");
                //sleep(Duration::from_secs(3));
                self.node_handler.network().send(endpoint, &output_data);
            }
            Message::Join(socket_addr) => {
                println!(
                    "{}: request from endpoint: {endpoint} ip: {socket_addr}",
                    self.self_addr
                );
                self.join_handler(endpoint, socket_addr);
                //Find the closest to that position
            }
            Message::Message(mex) => {
                println!("Message received from other peer");
                println!("My ip {:?}", self.self_addr);
                println!("{mex}")
            }
            Message::AddSuccessor(address) => {
                //println!("Add succesossr {endpoint}, {mex} {}", self.self_addr);
                self.finger_table.insert(0, address);

                println!("{}, {:?}", self.self_addr, self.finger_table);
            }
            Message::AddPredecessor(address) => self.predecessor = Some(address),
            Message::ForwardedJoin(socket_addr) => {
                println!("Oh shit. here we go again");

                let join_message = Message::Join(self.self_addr);
                let output_data = bincode::serialize(&join_message).unwrap();

                self.node_handler.network().remove(endpoint.resource_id());
                let mut y = true;

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

    fn join_handler(&mut self, endpoint: Endpoint, socket_addr: SocketAddr) {
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

            let message = bincode::serialize(&Message::AddSuccessor(self.self_addr)).unwrap();
            self.node_handler.network().send(*endpoint, &message);
            let message = bincode::serialize(&Message::AddPredecessor(self.self_addr)).unwrap();
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
            let role = if is_predecessor {
                "predecessor and self"
            } else {
                "self and successor"
            };
            println!("{}: Inserting between {}", self.self_addr, role);

            let first_message = if is_predecessor {
                Message::AddSuccessor(self.self_addr)
            } else {
                Message::AddPredecessor(self.self_addr)
            };
            let output_data = bincode::serialize(&first_message).unwrap();
            self.node_handler.network().send(*endpoint, &output_data);

            let second_message = if is_predecessor {
                Message::AddPredecessor(self.predecessor.unwrap())
            } else {
                Message::AddSuccessor(self.finger_table[0])
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

        let message = Message::ForwardedJoin(self.finger_table[e]);
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
