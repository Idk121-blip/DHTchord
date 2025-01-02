use crate::common;
use crate::common::{binary_search, ChordMessage, Message, ServerSignals, ServerToUserMessage, SERVER_FOLDER};
use crate::errors::GetError;
use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::network::Transport;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::fs::File;
use std::io::Read;
use std::ops::Add;
use tracing::trace;

pub fn handle_forwarded_get(handler: &NodeHandler<ServerSignals>, config: &NodeConfig, addr: String, key: String) {
    trace!("{addr}");
    let (endpoint, _) = handler.network().connect(Transport::Ws, &addr).unwrap();
    let message = ServerSignals::SendMessageToUser(endpoint, get_from_key(handler, config, addr, key));
    handler.signals().send(message);
}

pub fn get_from_key(
    handler: &NodeHandler<ServerSignals>,
    config: &NodeConfig,
    addr: String,
    key: String,
) -> ServerToUserMessage {
    match handle_user_get(handler, config, key.clone(), addr) {
        Ok(file) => ServerToUserMessage::RequestedFile(file),
        Err(e) => match e {
            GetError::ForwardingRequest(addr) => ServerToUserMessage::ForwarderTo(addr),
            GetError::ErrorRetrievingFile => ServerToUserMessage::InternalServerError,
            GetError::NotFound => ServerToUserMessage::FileNotFound(key),
            GetError::HexConversion => ServerToUserMessage::HexConversionNotValid(key),
        },
    }
}

fn handle_user_get(
    handler: &NodeHandler<ServerSignals>,
    config: &NodeConfig,
    key: String,
    addr: String,
) -> Result<common::File, GetError> {
    trace!("Handling user get");

    if let Ok(digested_file_name) = hex::decode(key.clone()) {
        let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

        if (digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
            || config.id > successor && digested_file_name < successor
            || config.finger_table.is_empty()
        {
            let file_name = config.saved_files.get(&key);

            if file_name.is_none() {
                trace!("No such a file");
                return Err(GetError::NotFound);
            }

            let file_path = SERVER_FOLDER
                .to_string()
                .add(&config.self_addr.port().to_string())
                .add("/")
                .add(&key);

            let buffer = get_file_bytes(file_path);

            let file = common::File {
                name: file_name.unwrap().to_string(),
                buffer,
            };

            trace!("returning {}", file_name.unwrap());
            return Ok(file);
        }

        let forwarding_index = binary_search(config, &digested_file_name); // todo check code duplication with put

        let (forwarding_endpoint, _) = handler
            .network()
            .connect(Transport::Ws, config.finger_table[forwarding_index])
            .unwrap();

        handler.signals().send(ServerSignals::ForwardMessage(
            forwarding_endpoint,
            Message::ChordMessage(ChordMessage::ForwardedGet(addr, key)),
        ));

        return Err(GetError::ForwardingRequest(
            config.finger_table[forwarding_index].to_string(),
        ));
    }

    Err(GetError::HexConversion)
}

pub fn get_file_bytes(file_path: String) -> Vec<u8> {
    trace!("{file_path}");

    let file = File::open(file_path);

    let mut buffer = Vec::new();

    let _ = file.unwrap().read_to_end(&mut buffer); //todo check that works fine

    buffer
}
