use crate::common::{binary_search, get_udp_endpoint, get_ws_endpoint, ChordMessage, Message, ServerSignals};
use crate::node_state::{NodeConfig, FINGER_TABLE_SIZE, HEARTBEAT_TIMEOUT, ID_BYTES, MAXIMUM_DURATION};
use chrono::Utc;
use message_io::node::NodeHandler;
use std::ops::Mul;
use tracing::trace;

fn binary_add(mut vec: Vec<u8>, index: usize, bytes: usize) -> Result<Vec<u8>, ()> {
    let mut byte = bytes - 1;
    let power = (index % 8) as u8;
    if (byte as isize - (index / 8) as isize) < 0 {
        return Err(());
    }

    byte -= index / 8;
    let mut specific_byte = vec[byte] as u16;

    let mut shifted_increment = 1;
    shifted_increment <<= power;

    specific_byte += shifted_increment;
    let mut carry;
    if specific_byte > FINGER_TABLE_SIZE as u16 {
        while specific_byte > FINGER_TABLE_SIZE as u16 {
            carry = specific_byte / (FINGER_TABLE_SIZE as u16 + 1);

            vec[byte] = (specific_byte % FINGER_TABLE_SIZE as u16) as u8;

            if byte as isize - 1 < 0 {
                break;
            }
            byte -= 1;
            specific_byte = carry + vec[byte] as u16;
        }
    } else {
        vec[byte] = specific_byte as u8;
    }
    Ok(vec)
}

pub fn stabilization_protocol(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig) {
    check_successor(handler, config);
    heart_beat(handler, config);
    update_finger_table(handler, config);
}

fn check_successor(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig) {
    if config.finger_table.is_empty() {
        return;
    }
    let now = Utc::now();
    if now.signed_duration_since(config.last_modified) > HEARTBEAT_TIMEOUT {
        if config.successors_cache.is_empty() {
            return;
        }
        while !config.successors_cache.is_empty() {
            if config.finger_table[0] != config.successors_cache[0] {
                config.finger_table[0] = config.successors_cache[0];
                if config.finger_table.len() > 1 && config.finger_table[0] == config.finger_table[1] {
                    config.finger_table.remove(1);
                }
            }
            config.successors_cache.remove(0);
        }
        let endpoint = get_ws_endpoint(handler, config, config.finger_table[0]);
        handler.signals().send(ServerSignals::ForwardMessage(
            endpoint,
            Message::ChordMessage(ChordMessage::NotifySuccessor(config.self_addr)),
        ));
    }
}

fn heart_beat(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig) {
    if config.predecessor.is_none() {
        return;
    }

    let endpoint = get_udp_endpoint(handler, config, config.predecessor.unwrap());

    let successor = if config.finger_table.is_empty() {
        config.self_addr
    } else {
        config.finger_table[0]
    };

    let message = Message::ChordMessage(ChordMessage::HeartBeat(config.self_addr, successor));

    handler.signals().send(ServerSignals::ForwardMessage(endpoint, message));
}

fn update_finger_table(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig) {
    if !config.finger_table.is_empty() {
        let searching = binary_add(config.id.clone(), 0, ID_BYTES).unwrap();
        let mut forwarding_index = binary_search(config, &searching);
        let mut socket_address = config.finger_table[forwarding_index];
        let mut endpoint = get_ws_endpoint(handler, config, socket_address);

        handler.signals().send(ServerSignals::ForwardMessage(
            endpoint,
            Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
        ));

        for i in 0..FINGER_TABLE_SIZE {
            let searching = binary_add(config.id.clone(), i, ID_BYTES).unwrap();
            forwarding_index = binary_search(config, &searching);

            socket_address = config.finger_table[forwarding_index];

            endpoint = get_ws_endpoint(handler, config, socket_address);

            handler.signals().send(ServerSignals::ForwardMessage(
                endpoint,
                Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
            ));
        }

        if config.gossip_interval.lt(&MAXIMUM_DURATION) {
            let new_interval = config.gossip_interval.mul(2);
            config.gossip_interval = new_interval;
        }
    }
    handler
        .signals()
        .send_with_timer(ServerSignals::Stabilization(), config.gossip_interval);
    trace!("{:?}", config.finger_table);
}
