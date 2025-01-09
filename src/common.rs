use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::network::{Endpoint, Transport};
use message_io::node::NodeHandler;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;

pub(crate) const SERVER_FOLDER: &str = "server/";

#[derive(Serialize, Deserialize)]
pub(crate) enum Message {
    ChordMessage(ChordMessage),
    UserMessage(UserMessage),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ChordMessage {
    NotifySuccessor(SocketAddr),

    NotifyPredecessor(SocketAddr),

    NotifyPresence(SocketAddr),

    AddSuccessor(SocketAddr),

    AddPredecessor(SocketAddr),

    Join(SocketAddr),

    Message(String),

    ForwardedJoin(SocketAddr),

    ForwardedPut(SocketAddr, File),

    ForwardedGet(SocketAddr, String),

    MoveFile(File),

    HeartBeat(SocketAddr, SocketAddr),

    Find(Vec<u8>, SocketAddr),
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
    HeartBeat(),
    Stabilization(),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum UserMessage {
    ///Put(file_to_save, self_address)
    Put(File, SocketAddr),
    ///Get(key, self_address)
    Get(String, SocketAddr),
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub name: String,
    pub buffer: Vec<u8>,
}

pub(crate) fn binary_search(config: &NodeConfig, digested_vector: &Vec<u8>) -> usize {
    if config.finger_table.is_empty() {
        return 0;
    }
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

pub(crate) fn get_ws_endpoint(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    socket_addr: SocketAddr,
) -> Endpoint {
    if let Entry::Vacant(e) = config.known_endpoints_ws.entry(socket_addr) {
        let (endpoint, _) = handler.network().connect(Transport::Ws, socket_addr).unwrap();
        e.insert(endpoint);
        endpoint
    } else {
        *config.known_endpoints_ws.get(&socket_addr).unwrap()
    }
}

pub(crate) fn get_udp_endpoint(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    socket_addr: SocketAddr,
) -> Endpoint {
    if let Entry::Vacant(e) = config.known_endpoints_udp.entry(socket_addr) {
        let (endpoint, _) = handler.network().connect(Transport::Udp, socket_addr).unwrap();
        e.insert(endpoint);
        endpoint
    } else {
        *config.known_endpoints_ws.get(&socket_addr).unwrap()
    }
}
