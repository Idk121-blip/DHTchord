use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub enum Message {
    ChordMessage(ChordMessage),
    UserMessage(UserMessage),
}

#[derive(Serialize, Deserialize)]
#[warn(private_interfaces)]
pub(crate) enum ChordMessage {
    SendStringMessage(String, SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    ForwardedJoin(String),

    ForwardedPut(String, File),
    // SendMessage(Box<Message<T>>, SocketAddr),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ServerToUserMessage {
    RequestedFile(File),
}

pub(crate) enum ServerSignals {
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
    pub buffer: Vec<u8>,
}
