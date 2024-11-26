use crate::common::Message::{self};
use digest::consts::U32;
use digest::core_api::{CoreWrapper, CtVariableCoreWrapper};
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use sha2::{Digest, OidSha256, Sha256, Sha256VarCore};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::thread::sleep;
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

        // A node_listener for any other participant that want to establish connection.
        // Returned 'listen_addr' contains the port that the OS gives for us when we put a 0.
        let listen_addr = &(ip_addr.to_string() + ":" + &port.to_string());
        //let Ok((_, listen_addr)) =
        handler.network().listen(Transport::FramedTcp, listen_addr).unwrap();
        // else { panic!("Pippa"); };

        println!("Discovery server running at {}", listen_addr);

        // let discovery_addr = "127.0.0.1:2000"; // Connection to the discovery server.
        // let Ok((endpoint, _)) = handler.network().connect(Transport::FramedTcp, discovery_addr)
        // else { panic!("Pippa2") };

        Self {
            node_handler: handler,
            id: vec![],
            node_listener: Some(node_listener),
            self_addr: SocketAddr::new(ip_addr, port),

            //endpoint,
            known_peers: Default::default(),
            finger_table: vec![],
            predecessor: None,
            gossip_interval: Default::default(),
            sha: Sha256::new(),
        }
    }

    //
    // pub fn set_sha(mut self)->Self{
    //     self.sha = sha;
    //     self
    // }

    pub fn run(mut self) {
        println!("{}", self.self_addr.port());
        if self.self_addr.port() == 8910 {
            sleep(Duration::from_secs(2));
            let (endpoint, _) = self
                .node_handler
                .network()
                .connect(Transport::FramedTcp, "127.0.0.1:8888")
                .unwrap();

            let mex2 = Message::Join(self.self_addr.clone());

            match bincode::serialize(&mex2) {
                Ok(output_data) => {
                    self.node_handler.network().send(endpoint, &output_data);
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
                println!("{}: request from ip: {endpoint} connected: {result}", self.self_addr)
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
            _ => {}
        }
    }

    fn join_handler(&mut self, endpoint: Endpoint, socket_addr: SocketAddr) {
        println!("entering join process");
        let mut hasher = Sha256::new();
        hasher.update(socket_addr.to_string().as_bytes());
        let node_id = hasher.clone().finalize().to_vec();

        // If the current node's finger table is empty, it's the only node in the network.
        if self.finger_table.is_empty() {
            println!("{}: request from ip: {endpoint} joining: {socket_addr}", self.self_addr);
            // Join directly as both successor and predecessor.
            self.predecessor = Some(socket_addr);
            self.finger_table.push(socket_addr);

            let message = bincode::serialize(&Message::AddSuccessor(self.self_addr)).unwrap();
            self.node_handler.network().send(endpoint, &message);
            let message = bincode::serialize(&Message::AddPredecessor(self.self_addr)).unwrap();
            self.node_handler.network().send(endpoint, &message);
            return;
        }

        // Compute this node's hashed ID, predecessor, and successor.
        hasher.update(self.finger_table[0].to_string().as_bytes());
        let successor = hasher.clone().finalize().to_vec();

        // let mut hasher = Sha256::new();
        hasher.update(self.predecessor.unwrap().to_string().as_bytes()); //todo check the unwrap
        let predecessor = hasher.clone().finalize().to_vec();

        if node_id > self.id && node_id <= successor {

            // The new node should join as this node's successor.
            let successor_message = Message::AddSuccessor(self.finger_table[0]);
            let output_data = bincode::serialize(&successor_message).unwrap();
            self.node_handler.network().send(endpoint, &output_data);
            return;
        }

        if node_id < self.id && node_id >= predecessor {
            // The new node should join as this node's predecessor.
            //self.notify_predecessor(socket_addr);
            return;
        }

        // Perform binary search in the finger table to find the closest node.
        self.binary_search(hasher.clone(), &node_id);
    }

    fn binary_search(
        &self,
        mut hasher: CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, U32, OidSha256>>,
        node_id: &Vec<u8>,
    ) {
        let mut s = 0;
        let mut e = self.finger_table.len();
        while s < e {
            let mid = (s + e) / 2;
            hasher.update(self.finger_table[mid].to_string().as_bytes());
            let mid_id = hasher.clone().finalize().to_vec();

            if mid_id > *node_id {
                e = mid;
            } else {
                s = mid + 1;
            }
        }
    }
}
