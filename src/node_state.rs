use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub struct NodeState {
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    //Check if endpoint is needed
    //endpoint: Endpoint,
    self_addr: SocketAddr, // The node's own address

    known_peers: HashSet<SocketAddr>, // Known peers in the network
    gossip_interval: Duration,   // Time between gossip rounds
}


impl NodeState {
    pub fn new()->Self{
        let (handler, node_listener) = node::split();

        // A node_listener for any other participant that want to establish connection.
        // Returned 'listen_addr' contains the port that the OS gives for us when we put a 0.
        let listen_addr = "127.0.0.1:8888";
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
            self_addr: SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8888".parse().unwrap()),
            //endpoint,
            known_peers: Default::default(),
            gossip_interval: Default::default(),
        }


    }

    pub fn run(mut self){
        // thread::spawn(move || {
        //     loop {
        //         //self.gossip_interval
        //     }
        // });
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event.network() {
            _ => println!("Se un pirla")
        });
    }



}