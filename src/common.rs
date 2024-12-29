use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
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
}

#[derive(Serialize, Deserialize)]
pub(crate) enum UserMessage {
    ///Put(file_to_save, self_address)
    Put(File, String),
    ///Get(key, self_address)
    Get(String, String),
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub name: String,
    pub buffer: Vec<u8>,
}


pub(crate) fn binary_search(config: &NodeConfig, digested_vector: &Vec<u8>) -> usize {
    let mut s = 0;
    let mut e = config.finger_table.len();
    while s < e {
        let mid = (s + e) / 2;
        let mid_id = Sha256::digest(config.finger_table[mid].to_string().as_bytes()).to_vec();

        if mid_id > *digested_vector {
            e = mid;
        } else {
            s = mid + 1;
        }
    }

    if e == config.finger_table.len() {
        e -= 1;
    }

    e
}