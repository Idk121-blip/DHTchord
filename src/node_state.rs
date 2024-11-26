use crate::common::Message;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use sha2::{Sha256, Digest};

pub struct NodeState {
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    //Check if endpoint is needed
    //endpoint: Endpoint,
    id: Vec<u8>,
    self_addr: SocketAddr, // The node's own address
    known_peers: HashSet<SocketAddr>, // Known peers in the network
    finger_table: Vec<SocketAddr>,
    predecessor: Option<SocketAddr>,
    gossip_interval: Duration,   // Time between gossip rounds
    sha: Sha256,
}


impl NodeState {
    pub fn new(ip_addr: IpAddr, port: u16)->Self{
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

        Self{
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

    pub fn run(mut self){
        println!("{}", self.self_addr.port());
        if self.self_addr.port()==8910{
            let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, "127.0.0.1:8888").unwrap();
            println!("{endpoint}");

            let mex2= Message::SendStringMessage("ciaooo".to_owned(), self.self_addr.clone());
            let output_data = bincode::serialize(&mex2).unwrap();
            self.node_handler.network().send(endpoint, &output_data);

        }
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event.network() {
            NetEvent::Message(endpoint, input_data)=>{
                let message: Message = bincode::deserialize(&input_data).unwrap(); // todo: work on this
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
                        let mex2= Message::Message(mex);
                        println!("Send message from {}, {}", addr, self.self_addr);
                        let output_data = bincode::serialize(&mex2).unwrap();

                        //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
                        println!("{endpoint}");
                        //sleep(Duration::from_secs(3));
                        self.node_handler.network().send(endpoint, &output_data);
                    }
                    Message::Join(socket_addr)=>{
                        let mut hasher = Sha256::new();
                        hasher.update(socket_addr.to_string().as_bytes());
                        let node_id = hasher.clone().finalize().to_vec();

                        // If the current node's finger table is empty, it's the only node in the network.
                        if self.finger_table.is_empty() {
                            // Join directly as both successor and predecessor.
                            self.predecessor = Some(socket_addr);
                            self.finger_table.push(socket_addr);
                            return;
                        }

                        // Compute this node's hashed ID, predecessor, and successor.
                        hasher.update(self.finger_table[0].to_string().as_bytes());
                        let successor = hasher.clone().finalize().to_vec();

                        // let mut hasher = Sha256::new();
                        hasher.update(self.predecessor.unwrap().to_string().as_bytes());//todo check the unwrap
                        let predecessor = hasher.clone().finalize().to_vec();

                        if node_id > self.id && node_id <= successor {
                            // The new node should join as this node's successor.
                            //self.notify_successor(socket_addr);
                            return;
                        }

                        if node_id < self.id && node_id >= predecessor {
                            // The new node should join as this node's predecessor.
                            //self.notify_predecessor(socket_addr);
                            return;
                        }

                        // Perform binary search in the finger table to find the closest node.
                        let mut s = 0;
                        let mut e = self.finger_table.len();
                        while s < e {
                            let mid = (s + e) / 2;

                            hasher.update(self.finger_table[mid].to_string().as_bytes());
                            let mid_id = hasher.clone().finalize().to_vec();

                            if mid_id > node_id {
                                e = mid;
                            } else {
                                s = mid + 1;
                            }
                        }

                        //Find the closest to that position

                        
                    }
                    Message::Message(mex) => {
                        println!("Message received from other peer");
                        println!("My ip {:?}", self.self_addr);
                        println!("{mex}")
                    }
                    _=>{}
                }
            }

            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Disconnected(_) => {}
        });
    }





}