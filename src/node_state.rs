use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::thread::sleep;
use std::time::Duration;
use crate::common::Message;

pub struct NodeState {
    node_handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    //Check if endpoint is needed
    //endpoint: Endpoint,
    self_addr: SocketAddr, // The node's own address
    coso: usize,
    known_peers: HashSet<SocketAddr>, // Known peers in the network
    gossip_interval: Duration,   // Time between gossip rounds
}


impl NodeState {
    pub fn new(ip_addr: IpAddr, port: u16)->Self{
        let (handler, node_listener) = node::split();

        // A node_listener for any other participant that want to establish connection.
        // Returned 'listen_addr' contains the port that the OS gives for us when we put a 0.
        let listen_addr = "127.0.0.1:8888";
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
            coso: 0
        }


    }

    pub fn run(mut self){
        // thread::spawn(move || {
        //     loop {
        //         //self.gossip_interval
        //     }
        // });
        println!("{}", self.self_addr.port());
        if self.self_addr.port()==8910 && self.coso == 0{
            println!("ciao");
            let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, "127.0.0.1:8888").unwrap();
            println!("{endpoint}");

            let mex2= Message::SendMessage("ciaooo".to_owned(), self.self_addr.clone());
            println!("ciao2");
            let output_data = bincode::serialize(&mex2).unwrap();
            self.node_handler.network().send(endpoint, &output_data);
            self.coso+=1;
        }
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event.network() {
            NetEvent::Message(endpoint, input_data)=>{
                let message: Message = bincode::deserialize(&input_data).unwrap();
                match message {
                    Message::RegisterServer(_, _) => {}
                    Message::ScanningFor(_, _) => {}
                    Message::ServerAdded(_, _) => {}
                    Message::SendMessage(mex, addr) => {
                        let mex2= Message::Message(mex);
                        println!("Send message from {}", addr);
                        let output_data = bincode::serialize(&mex2).unwrap();

                        let ip= addr.ip().to_string()+ ":" + &*addr.port().to_string();

                        //let (endpoint, _ )= self.node_handler.network().connect(Transport::FramedTcp, &*ip).unwrap();
                        println!("{endpoint}");
                        sleep(Duration::from_secs(3));
                        self.node_handler.network().send(endpoint, &output_data);


                    }
                    Message::Message(mex) => {
                        println!("Message received from other peer");
                        println!("My ip {:?}", self.self_addr);
                        println!("{mex}")
                    }
                }
            }
            _ => println!("Sei un pirla {}", self.self_addr)
        });
    }



}