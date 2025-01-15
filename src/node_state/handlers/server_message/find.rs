use crate::common::{binary_search, get_endpoint, ChordMessage, Message, ServerSignals};
use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::net::SocketAddr;

pub fn handle_find(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, wanted_id: Vec<u8>, searching_address: SocketAddr) {
    if wanted_id == config.id {
        let searching_endpoint = get_endpoint(handler, config, searching_address);
        handler.signals().send(ServerSignals::ForwardMessage(
            searching_endpoint,
            Message::ChordMessage(ChordMessage::NotifyPresence(config.self_addr)),
        ));
        return;
    }

    let index = binary_search(config, &wanted_id);

    let digested_address = Sha256::digest(config.finger_table[index].to_string().as_bytes()).to_vec();

    if digested_address == wanted_id {
        //iterative way
        let searching_endpoint = get_endpoint(handler, config, searching_address);
        handler.signals().send(ServerSignals::ForwardMessage(
            searching_endpoint,
            Message::ChordMessage(ChordMessage::NotifyPresence(config.finger_table[index])),
        ));
        return;
    }

    let digested_ip_address_request = Sha256::digest(searching_address.to_string().as_bytes()).to_vec();

    //3 cases 1) if we are returning to the "starting" point, 2) if the node doesn't exist
    // 3) if we are restarting the circle (9->0, and we are looking for 10)
    if (config.id > wanted_id
        && (digested_ip_address_request < wanted_id
        || wanted_id < digested_address
        || digested_address < config.id))
        || digested_ip_address_request == digested_address
    {
        //not found no need to send a response since it will increase the traffic
        return;
    }

    let forwarding_address = config.finger_table[index];

    let forwarding_endpoint = get_endpoint(handler, config, forwarding_address);

    handler.signals().send(ServerSignals::ForwardMessage(
        forwarding_endpoint,
        Message::ChordMessage(ChordMessage::Find(wanted_id, searching_address)),
    ));
}