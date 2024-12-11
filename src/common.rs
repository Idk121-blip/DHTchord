use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub enum ChordMessage {
    //<T>
    RegisterServer(String, SocketAddr),

    ScanningFor(SocketAddr, SocketAddr),

    ServerAdded(String, SocketAddr),

    SendStringMessage(String, SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    Accepted,

    Rejected,

    ForwardedJoin(String),
    // SendMessage(Box<Message<T>>, SocketAddr),
}


pub(crate) enum Signals {
    ForwardedJoin(Endpoint)
}