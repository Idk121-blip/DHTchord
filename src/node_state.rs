use crate::common::Message::{self};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub struct NodeState {
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    //Check if endpoint is needed
    //endpoint: Endpoint,
    id: Vec<u8>,
    self_addr: SocketAddr,            // The node's own address
    known_peers: HashSet<SocketAddr>, // Known peers in the network
    finger_table: Vec<SocketAddr>,
    predecessor: Option<SocketAddr>,
    gossip_interval: Duration, // Time between gossip rounds
    sha: Sha256,
}

impl NodeState {
    pub fn new(ip_addr: IpAddr, port: u16) -> Self {
        let (handler, node_listener) = node::split();

        let listen_addr = &(ip_addr.to_string() + ":" + &port.to_string());
        handler.network().listen(Transport::FramedTcp, listen_addr).unwrap();

        println!("Discovery server running at {}", listen_addr);

        let id = Sha256::digest(listen_addr.to_string().as_bytes()).to_vec();
        println!("{:?}", id);
        Self {
            node_handler: handler,
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
        println!("{}", self.self_addr.port());
        if self.self_addr.port() == 8888 {
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
            println!("works fine");
            let mex2 = Message::Join(self.self_addr.clone());
            println!("Digddffd {}", self.self_addr.port());
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
            NetEvent::Connected(endpoint, result) => {
                //println!("{}: request from ip: {endpoint} connected: {result}", self.self_addr)
            }
            NetEvent::Accepted(_, _) => {}
            NetEvent::Disconnected(_) => {}
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
            Message::RegisterServer(x1, x2) => {
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
                println!("{}: request from ip: {endpoint} joining: {socket_addr}", self.self_addr);
                self.join_handler(endpoint, socket_addr);
                //Find the closest to that position
            }
            Message::Message(mex) => {
                println!("Message received from other peer");
                println!("My ip {:?}", self.self_addr);
                println!("{mex}")
            }
            Message::AddSuccessor(mex) => {
                //println!("Add succesossr {endpoint}, {mex} {}", self.self_addr);
                //todo
            }
            Message::AddPredecessor(mex) => {
                //println!("Add succesossr {endpoint}, {mex} {}", self.self_addr);
                //todo
            }
            Message::ForwardedJoin(_) => {}

            _ => {}
        }
    }

    fn join_handler(&mut self, endpoint: Endpoint, socket_addr: SocketAddr) {
        println!("entering join process");

        let node_id = Sha256::digest(socket_addr.to_string().as_bytes()).to_vec();

        if self.has_empty_table(&endpoint, &socket_addr) {
            println!("Node added to empty table");
            return;
        }

        let successor = Sha256::digest(self.finger_table[0].to_string().as_bytes()).to_vec();

        let predecessor = Sha256::digest(self.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

        if self.insert_near_self(&endpoint, &socket_addr, &node_id, successor, predecessor) {
            return;
        }

        self.binary_search(&node_id);
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

    fn binary_search(&self, node_id: &Vec<u8>) {
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

        //todo Check if it's closer this one or the one that is predecessor
        // NB: it won't be unless mid = e at the end (CREDO)
    }
}
