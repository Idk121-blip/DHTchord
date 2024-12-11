use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
pub enum Message
// <T>
// where
//     T: Serialize,
{
    ChordMessage(ChordMessage),
    UserMessage(UserMessage<isize>),
    // Phantom(T),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ChordMessage {
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
    ForwardedJoin(Endpoint),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UserMessage<T>
where
    T: Serialize,
{
    Put(T),
    Get(T),
}
