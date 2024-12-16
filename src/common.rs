use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub enum Message {
    ChordMessage(ChordMessage),
    UserMessage(UserMessage),
    // Phantom(T),
}

#[derive(Serialize, Deserialize)]
#[warn(private_interfaces)]
pub(crate) enum ChordMessage {
    //<T>
    SendStringMessage(String, SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    ForwardedJoin(String),
    // SendMessage(Box<Message<T>>, SocketAddr),
}

pub(crate) enum Signals {
    ForwardedJoin(Endpoint),
    ForwardPut(Endpoint, File),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UserMessage {
    ///Put(file bytes, file name,  extension)
    Put(File),
    Get(Vec<u8>),
}
#[derive(Serialize, Deserialize, Debug)]
pub struct File {
    pub name: String,
    pub extension: String,
    pub data: Vec<u8>,
}
