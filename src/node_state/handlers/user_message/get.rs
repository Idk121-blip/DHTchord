use crate::common;
use crate::common::{
    binary_search, get_endpoint, ChordMessage, Message, ServerSignals, ServerToUserMessage, SERVER_FOLDER,
};
use crate::errors::GetError;
use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::ops::Add;
use tracing::trace;

pub fn handle_forwarded_get(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    addr: SocketAddr,
    key: String,
) {
    let endpoint = get_endpoint(handler, config, addr);
    let message = ServerSignals::SendMessageToUser(endpoint, get_from_key(handler, config, addr, key));
    handler.signals().send(message);
}

pub fn get_from_key(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    addr: SocketAddr,
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
    config: &mut NodeConfig,
    key: String,
    addr: SocketAddr,
) -> Result<common::File, GetError> {
    trace!("Handling user get");

    if hex::decode(key.clone()).is_err() {
        return Err(GetError::HexConversion);
    }

    let digested_file_name = hex::decode(key.clone()).unwrap();
    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if !(digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
        && !(config.id > successor && digested_file_name < successor)
        && !config.finger_table.is_empty()
    {
        let forwarding_index = binary_search(config, &digested_file_name); // todo check code duplication with put
        let forwarding_address = config.finger_table[forwarding_index];

        let forwarding_endpoint = get_endpoint(handler, config, forwarding_address);

        handler.signals().send(ServerSignals::ForwardMessage(
            forwarding_endpoint,
            Message::ChordMessage(ChordMessage::ForwardedGet(addr, key)),
        ));

        return Err(GetError::ForwardingRequest(
            config.finger_table[forwarding_index].to_string(),
        ));
    }

    if !config.saved_files.contains_key(&key) {
        trace!("No such a file");
        return Err(GetError::NotFound);
    }

    let file_name = config.saved_files.get(&key).unwrap();

    let file_path = SERVER_FOLDER
        .to_string()
        .add(&config.self_addr.port().to_string())
        .add("/")
        .add(&key);

    let buffer = get_file_bytes(file_path);

    let file = common::File {
        name: file_name.to_string(),
        buffer,
    };

    trace!("returning {}", file_name);
    Ok(file)
}

pub fn get_file_bytes(file_path: String) -> Vec<u8> {
    trace!("{file_path}");

    let file = File::open(file_path);

    let mut buffer = Vec::new();

    let _ = file.unwrap().read_to_end(&mut buffer); //todo check that works fine

    buffer
}
