use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Message{
    RegisterServer(String, SocketAddr),

    ScanningFor(SocketAddr, SocketAddr),

    ServerAdded(String, SocketAddr),

    Message(String)
}