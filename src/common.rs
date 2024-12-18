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

    ForwardedPut(String, File),
    // SendMessage(Box<Message<T>>, SocketAddr),
}

pub(crate) enum ServerMessage {
    RequestedFile(File),
}

pub(crate) enum Signals {
    ForwardMessage(Endpoint, Message),
    ForwardPut(Endpoint, File),
}

#[derive(Serialize, Deserialize)]
pub enum UserMessage {
    ///Put(file bytes, file name,  extension)
    Put(File),
    Get(String),
}
#[derive(Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub extension: String,
    pub data: Vec<u8>,
}
