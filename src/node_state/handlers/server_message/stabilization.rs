use crate::common::{binary_search, get_endpoint, ChordMessage, Message, ServerSignals};
use crate::node_state::{NodeConfig, FINGER_TABLE_SIZE, ID_BYTES, MAXIMUM_DURATION};
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

    let mut x = 1;
    x <<= power;

    specific_byte += x;
    let mut carry;
    if specific_byte > 255 {
        while specific_byte > 255 {
            carry = specific_byte / 255;

            vec[byte] = (specific_byte % 255) as u8;

            if byte as isize - 1 < 0 {
                break;
            }
            byte -= 1;
            specific_byte = carry + vec[byte] as u16;
        }
    } else {
        vec[byte] = (specific_byte % 255) as u8;
    }
    Ok(vec)
}

pub fn stabilization_protocol(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig) {
    //TODO create a function to wrap this
    trace!("Stabilization");
    if config.gossip_interval.lt(&MAXIMUM_DURATION) {
        let new_interval = config.gossip_interval.mul(2);
        config.gossip_interval = new_interval;
    }

    let searching = binary_add(config.id.clone(), 0, ID_BYTES).unwrap();
    let mut forwarding_index = binary_search(config, &searching);

    let socket_address = config.finger_table[forwarding_index];

    let mut endpoint = get_endpoint(handler, config, socket_address);

    handler.signals().send(ServerSignals::ForwardMessage(
        endpoint,
        Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
    ));

    for i in 0..FINGER_TABLE_SIZE {
        let searching = binary_add(config.id.clone(), i, ID_BYTES).unwrap();
        let new_forwarding_index = binary_search(config, &searching);

        let socket_address = config.finger_table[new_forwarding_index];

        endpoint = get_endpoint(handler, config, socket_address);

        handler.signals().send(ServerSignals::ForwardMessage(
            endpoint,
            Message::ChordMessage(ChordMessage::Find(searching, config.self_addr)),
        ));
    }

    //trace!("{:?}", config.finger_table);
    handler
        .signals()
        .send_with_timer(ServerSignals::Stabilization(), config.gossip_interval);
}
