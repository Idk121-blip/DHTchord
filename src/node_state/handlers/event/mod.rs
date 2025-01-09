use crate::common::{get_udp_endpoint, ChordMessage, Message, ServerSignals};
use crate::node_state::handlers::server_message::handle_server_message;
use crate::node_state::handlers::server_message::stabilization::stabilization_protocol;
use crate::node_state::handlers::user_message::handle_user_message;
use crate::node_state::{NodeConfig, HEART_BEAT};
use message_io::network::{NetEvent, SendStatus};
use message_io::node::NodeHandler;
use tracing::trace;

pub fn signal_handler(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, signal: ServerSignals) {
    match signal {
        ServerSignals::ForwardMessage(endpoint, message) => {
            //trace!("Forwarding internal message");

            let output_data = bincode::serialize(&message).unwrap();

            if handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                //trace!(" Waiting for response {}", endpoint);
                handler.signals().send(ServerSignals::ForwardMessage(endpoint, message));
            }
        }
        ServerSignals::SendMessageToUser(endpoint, message) => {
            trace!("Forwarding message to user");

            let output_data = bincode::serialize(&message).unwrap();

            if handler.network().send(endpoint, &output_data) == SendStatus::ResourceNotAvailable {
                trace!("Waiting for response {}", endpoint);
                handler
                    .signals()
                    .send(ServerSignals::SendMessageToUser(endpoint, message));
            }
        }
        ServerSignals::Stabilization() => {
            trace!("Stabilization");
            stabilization_protocol(handler, config);
        }
        ServerSignals::HeartBeat() => {
            handler
                .signals()
                .send_with_timer(ServerSignals::HeartBeat(), HEART_BEAT);
            if config.predecessor.is_none() {
                return;
            }

            let endpoint = get_udp_endpoint(handler, config, config.predecessor.unwrap());

            let successor_address = if config.finger_table.is_empty() {
                config.self_addr
            } else {
                config.finger_table[0]
            };

            handler.signals().send(ServerSignals::ForwardMessage(
                endpoint,
                Message::ChordMessage(ChordMessage::HeartBeat(config.self_addr, successor_address)),
            ));
        }
    }
}
pub fn net_handler(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, net_event: NetEvent) {
    match net_event {
        NetEvent::Message(endpoint, serialized) => {
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
