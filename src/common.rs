use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub(crate) enum Message {
    ChordMessage(ChordMessage),
    UserMessage(UserMessage),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ChordMessage {
    SendStringMessage(String, SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    ForwardedJoin(String),

    ForwardedPut(String, File),

    ForwardedGet(String, String),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ServerToUserMessage {
    RequestedFile(File),
    SavedKey(String),
    ForwarderTo(String),
    FileNotFound(String),
    HexConversionNotValid(String),
    InternalServerError,
}

pub(crate) enum ServerSignals {
    ForwardMessage(Endpoint, Message),
    SendMessageToUser(Endpoint, ServerToUserMessage),
    ForwardPut(Endpoint, File, String),
    ForwardGet(Endpoint, String, String),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum UserMessage {
    ///Put(file_to_save, self_address)
    Put(File, String),
    ///Get(key, self_address)
    Get(String, String),
}
#[derive(Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub buffer: Vec<u8>,
}
