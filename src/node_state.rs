use crate::common::Message;
use digest::Digest;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use sha2::{Sha256};

pub struct NodeState <D> where D: Digest{
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    //Check if endpoint is needed
    //endpoint: Endpoint,
    self_addr: SocketAddr, // The node's own address
    known_peers: HashSet<SocketAddr>, // Known peers in the network
    gossip_interval: Duration,   // Time between gossip rounds
    sha: D
}


impl<D> NodeState <D> where D: Digest {
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
            node_listener: Some(node_listener),
            self_addr: SocketAddr::new(ip_addr, port),
            //endpoint,
            known_peers: Default::default(),
            gossip_interval: Default::default(),
            sha: Sha256::new(),
        }
    }


    pub fn set_sha(mut self, sha: D)->Self{
        self.sha = sha;
        self
    }

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
                    Message::Join(self_address)=>{
                        self.sha.clone().chain_update(self_address).finalize()
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