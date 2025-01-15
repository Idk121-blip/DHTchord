pub mod get;
pub mod put;

use crate::common::{ServerSignals, UserMessage};
use crate::node_state::handlers::user_message::get::get_from_key;
use crate::node_state::handlers::user_message::put::put_user_file;
use crate::node_state::NodeConfig;
use message_io::network::Endpoint;
use message_io::node::NodeHandler;

pub fn handle_user_message(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: UserMessage,
) {
    let message_to_send = match message {
        UserMessage::Put(file, user_addr) => put_user_file(handler, config, file, user_addr),
        UserMessage::Get(key, user_addr) => get_from_key(handler, config, user_addr, key),
    };
    let serialized = bincode::serialize(&message_to_send).unwrap();
    handler.network().send(endpoint, &serialized);
}
