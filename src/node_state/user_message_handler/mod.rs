pub mod get_handler;
pub mod put_handler;

use crate::common::{ServerSignals, UserMessage};
use crate::node_state::user_message_handler::get_handler::get_from_key;
use crate::node_state::user_message_handler::put_handler::put_user_file;
use crate::node_state::NodeConfig;
use message_io::network::Endpoint;
use message_io::node::NodeHandler;
use tracing::trace;

pub fn handle_user_message(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    endpoint: Endpoint,
    message: UserMessage,
) {
    let send_message = match message {
        UserMessage::Put(file, user_addr) => {
            trace!("Received file");
            put_user_file(handler, config, file, user_addr)
        }
        UserMessage::Get(key, user_addr) => {
            get_from_key(handler, config, user_addr, key)
        }
    };
    handler.network().send(endpoint, &bincode::serialize(&send_message).unwrap());
}