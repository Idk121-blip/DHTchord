use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub enum Message{//<T>
    RegisterServer(String, SocketAddr),

    ScanningFor(SocketAddr, SocketAddr),

    ServerAdded(String, SocketAddr),

    SendStringMessage(String, SocketAddr),

    // SendMessage(Box<Message<T>>, SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    Accepted,

    Rejected,

}