use crate::common::{Message, ServerSignals};
use crate::node_state::handlers::server_message::handle_server_message;
use crate::node_state::handlers::server_message::stabilization::stabilization_protocol;
use crate::node_state::handlers::user_message::handle_user_message;
use crate::node_state::NodeConfig;
use message_io::network::{Endpoint, NetEvent, SendStatus};
use message_io::node::NodeHandler;
use serde::Serialize;
use std::net::SocketAddr;
use tracing::trace;

pub fn signal_handler(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, signal: ServerSignals) {
    match signal {
        ServerSignals::ForwardMessage(endpoint, message) => {
            //trace!("Forwarding internal message");

            let output_data = bincode::serialize(&message).unwrap();

            if handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                trace!(" Waiting for response {}", endpoint);
                handler.signals().send(ServerSignals::ForwardMessage(endpoint, message))
            }

            //forward_message(&handler, endpoint, message, config.self_addr);
        }
        ServerSignals::SendMessageToUser(endpoint, message) => {
            trace!("Forwarding message to user");
            forward_message(handler, endpoint, message, config.self_addr);
        }
        ServerSignals::Stabilization() => {
            stabilization_protocol(handler, config);
        }
    }
}
pub fn net_handler(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, net_event: NetEvent) {
    match net_event {
        NetEvent::Message(endpoint, serialized) => {
            //
            let message = bincode::deserialize(serialized).unwrap();

            match message {
                Message::UserMessage(user_message) => {
                    trace!("Received user message");
                    handle_user_message(handler, config, endpoint, user_message);
                }
                Message::ChordMessage(server_message) => {
                    handle_server_message(handler, config, endpoint, server_message);
                }
            }
        }
        NetEvent::Connected(endpoint, result) => {
            trace!("request from ep: {endpoint} connected: {result}");
        }
        NetEvent::Accepted(_, _) => {
            trace!("Communication accepted");
        }
        NetEvent::Disconnected(endpoint) => {
            handler.network().remove(endpoint.resource_id());
        }
    }
}

fn forward_message(
    handler: &NodeHandler<ServerSignals>,
    endpoint: Endpoint,
    message: impl Serialize,
    addr: SocketAddr,
) {
    let output_data = bincode::serialize(&message).unwrap();

    while handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
        trace!("{} Waiting for response {}", addr, endpoint);
    }
}