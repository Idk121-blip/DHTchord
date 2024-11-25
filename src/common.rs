use std::net::SocketAddr;
use message_io::network::Endpoint;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Message{
    RegisterServer(String, SocketAddr),

    ScanningFor(SocketAddr, SocketAddr),

    ServerAdded(String, SocketAddr),

    SendMessage(String, SocketAddr),

    Message(String)
}