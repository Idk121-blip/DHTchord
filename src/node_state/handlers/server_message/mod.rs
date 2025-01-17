use crate::common;
use crate::common::{binary_search, get_endpoint, ChordMessage, Message, ServerSignals, SERVER_FOLDER};
use crate::node_state::handlers::server_message::find::handle_find;
use crate::node_state::handlers::user_message::get::{get_file_bytes, handle_forwarded_get};
use crate::node_state::handlers::user_message::put::{handle_forwarded_put, save_in_server};
use crate::node_state::NodeConfig;
use digest::Digest;
use join::handle_join;
use message_io::network::Endpoint;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::fs;
use std::net::SocketAddr;
use tracing::trace;

pub mod join;
pub mod stabilization;
mod find;

pub fn handle_server_message(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: ChordMessage,
) {
    match message {
        ChordMessage::AddPredecessor(predecessor) => {
            config.predecessor = Some(predecessor);

            let forwarding_endpoint = get_endpoint(handler, config, predecessor);

            if forwarding_endpoint.addr() != endpoint.addr() {
                handler.signals().send(ServerSignals::ForwardMessage(
                    forwarding_endpoint,
                    Message::ChordMessage(ChordMessage::NotifyPredecessor(config.self_addr)),
                ));
            }
        }
        ChordMessage::Join(addr) => {
            trace!("request from endpoint: {endpoint} ip: {addr}",);
            handle_join(handler, config, endpoint, addr);
            //Find the closest to that position
        }
        ChordMessage::Message(message) => {
            trace!("Message received from other peer: {message}");
        }
        ChordMessage::AddSuccessor(successor) => {
            //trace!("Add successor {endpoint}, {mex} {}", self.config.self_addr);
            config.finger_table.insert(0, successor);
            move_files(handler, config, successor, &endpoint);

            let forwarding_endpoint = get_endpoint(handler, config, successor);

            if forwarding_endpoint.addr() == endpoint.addr() {
                return;
            }

            // handler.signals().send(ServerSignals::ForwardMessage(
            //     forwarding_endpoint,
            //     Message::ChordMessage(ChordMessage::NotifySuccessor(config.self_addr)),
            // ));

            trace!("{}, {:?}", config.self_addr, config.finger_table);
        }
        ChordMessage::ForwardedJoin(addr) => {
            trace!("Forwarded join, joining {addr}");

            let new_endpoint = get_endpoint(handler, config, addr);
            let message = Message::ChordMessage(ChordMessage::Join(config.self_addr));
            handler
                .signals()
                .send(ServerSignals::ForwardMessage(new_endpoint, message));
        }
        ChordMessage::ForwardedPut(addr, file) => {
            trace!("Forwarded put");
            handle_forwarded_put(handler, config, addr, file);
        }
        ChordMessage::ForwardedGet(addr, key) => {
            trace!("Forwarded get");
            handle_forwarded_get(handler, config, addr, key);
        }
        ChordMessage::MoveFile(file) => {
            let _ = save_in_server(file, config.self_addr.port(), config);
        }

        ChordMessage::NotifySuccessor(predecessor) => {
            if config.predecessor.unwrap() == predecessor {
                return;
            }
            config.predecessor = Some(predecessor);
        }
        ChordMessage::NotifyPredecessor(successor) => {
            if config.finger_table[0] == successor {
                return;
            }
            config.finger_table.insert(0, successor);

            move_files(handler, config, successor, &endpoint);
            //todo remove the first one if it's not n+2^i id
        }
        ChordMessage::Find(wanted_id, searching_address) => {
            handle_find(handler, config, wanted_id, searching_address);
        }
        ChordMessage::NotifyPresence(addr) => {
            let digested_address = Sha256::digest(addr.to_string().as_bytes()).to_vec();
            let index = binary_search(config, &digested_address);
            trace!("{:?}\n {:?}", digested_address, config.id);
            if config.finger_table[index] == addr {
                return;
            }
            config.finger_table.insert(index, addr);
            trace!("Node added to finger table ");
        }
    }
}

fn move_files(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    new_node_addr: SocketAddr,
    endpoint: &Endpoint,
) {
    let digested_addr = Sha256::digest(new_node_addr.to_string().as_bytes()).to_vec();

    let forward_endpoint = get_endpoint(handler, config, new_node_addr);

    trace!("{} {}", endpoint.addr(), forward_endpoint.addr());

    for (key, file_name) in &config.saved_files {
        let digested_key = hex::decode(key).unwrap();
        if digested_key > digested_addr {
            //todo move the file that is saved

            let file_path = SERVER_FOLDER.to_string() + config.self_addr.port().to_string().as_str() + "/" + key;
            let buffer = get_file_bytes(file_path.clone());
            handler.signals().send(ServerSignals::ForwardMessage(
                forward_endpoint,
                Message::ChordMessage(ChordMessage::MoveFile(common::File {
                    name: file_name.to_string(),
                    buffer,
                })),
            ));
            fs::remove_file(file_path).unwrap()
        }
    }
}
