use crate::common::{binary_search, ChordMessage, Message, ServerSignals};
use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::network::{Endpoint, SendStatus};
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::net::SocketAddr;
use tracing::trace;

pub fn handle_join(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, endpoint: Endpoint, addr: SocketAddr) {
    trace!("entering join process");

    if config.finger_table.is_empty() {
        insert_in_empty_table(handler, config, &endpoint, &addr);
        trace!("Node added to empty table");
        return;
    }

    let node_id = Sha256::digest(addr.to_string().as_bytes()).to_vec();
    let predecessor = Sha256::digest(config.predecessor.unwrap().to_string().as_bytes()).to_vec(); //todo check the unwrap

    if node_id < config.id && node_id >= predecessor {
        trace!("Inserting between predecessor and self");
        insert_between_self_and_predecessor(handler, config, &endpoint, &addr);
        return;
    }

    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if node_id > config.id && (config.id > successor || node_id <= successor) {
        trace!("Inserting between self and successor");
        insert_between_self_and_successor(handler, config, &endpoint, &addr);
        return;
    }

    trace!("Starting forwarding process");

    forward_request(handler, config, &node_id, &endpoint);
}


fn insert_in_empty_table(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    config.predecessor = Some(*addr);
    config.finger_table.push(*addr);

    let message = Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);

    let message = Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr));
    let serialized = bincode::serialize(&message).unwrap();
    handler.network().send(*endpoint, &serialized);
    trace!("join successfully");
}

fn insert_between_self_and_predecessor(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    let add_successor_message = Message::ChordMessage(ChordMessage::AddSuccessor(config.self_addr));
    let serialized = bincode::serialize(&add_successor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    let add_predecessor_message = Message::ChordMessage(ChordMessage::AddPredecessor(config.predecessor.unwrap()));
    let serialized = bincode::serialize(&add_predecessor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    config.predecessor = Some(*addr);
    trace!("join successfully");
}

fn insert_between_self_and_successor(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: &Endpoint,
    addr: &SocketAddr,
) {
    //todo save in cache the successor 4 security reason
    let add_predecessor_message = Message::ChordMessage(ChordMessage::AddPredecessor(config.self_addr));
    let serialized = bincode::serialize(&add_predecessor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    let add_successor_message = Message::ChordMessage(ChordMessage::AddSuccessor(config.finger_table[0]));
    let serialized = bincode::serialize(&add_successor_message).unwrap();
    handler.network().send(*endpoint, &serialized);
    config.finger_table.insert(0, *addr);
    trace!("join successfully");
}

fn forward_request(handler: &NodeHandler<ServerSignals>, config: &NodeConfig, node_id: &Vec<u8>, endpoint: &Endpoint) {
    let forward_position = binary_search(config, node_id);
    let message = Message::ChordMessage(ChordMessage::ForwardedJoin(
        config.finger_table[forward_position].to_string(),
    ));
    let serialized = bincode::serialize(&message).unwrap();

    while handler.network().send(*endpoint, &serialized) == SendStatus::ResourceNotAvailable {
        trace!("Waiting for response...");
    }
}
