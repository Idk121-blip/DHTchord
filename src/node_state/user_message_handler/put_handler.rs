use crate::common;
use crate::common::{binary_search, ChordMessage, Message, ServerSignals, ServerToUserMessage};
use crate::errors::PutError;
use crate::node_state::NodeConfig;
use digest::Digest;
use message_io::network::Transport;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::fs::File;
use std::io::Write;
use std::ops::Add;
use std::path::Path;
use std::{fs, io};
use tracing::trace;

pub fn handle_forwarded_put(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, addr: String, file: common::File) {
    let (ep, _) = handler.network().connect(Transport::Ws, &addr).unwrap();
    let message = ServerSignals::SendMessageToUser(ep, put_user_file(handler, config, file, addr));
    handler.signals().send(message);
}

fn handle_user_put(
    handler: &NodeHandler<ServerSignals>,
    file: common::File,
    config: &mut NodeConfig,
    addr: String,
) -> Result<String, PutError> {
    let digested_file_name = Sha256::digest(file.name.as_bytes()).to_vec();
    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if (digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
        || config.id > successor && digested_file_name < successor
        || config.finger_table.is_empty()
    {
        return save_in_server(file, config.self_addr.port(), config)
            .map_or(Err(PutError::ErrorStoringFile), Ok);
    }

    let forwarding_index = binary_search(config, &digested_file_name);

    let (forwarding_endpoint, _) = handler
        .network()
        .connect(Transport::Ws, config.finger_table[forwarding_index])
        .unwrap();

    handler
        .signals()
        .send(ServerSignals::ForwardMessage(forwarding_endpoint, Message::ChordMessage(ChordMessage::ForwardedPut(addr, file))));

    Err(PutError::ForwardingRequest(
        config.finger_table[forwarding_index].to_string(),
    ))
}

pub fn put_user_file(handler: &NodeHandler<ServerSignals>, config: &mut NodeConfig, file: common::File, user_addr: String) -> ServerToUserMessage {
    match handle_user_put(handler, file, config, user_addr) {
        Ok(saved_key) => ServerToUserMessage::SavedKey(saved_key),
        Err(error) => match error {
            PutError::ForwardingRequest(address) => ServerToUserMessage::ForwarderTo(address),
            PutError::ErrorStoringFile => ServerToUserMessage::InternalServerError
        },
    }
}


fn save_in_server(file: common::File, port: u16, config: &mut NodeConfig) -> io::Result<String> {
    let common::File { name, buffer: data } = file;


    let digested_hex_file_name = hex::encode(Sha256::digest(name.as_bytes()));

    trace!("hashed_file_name: {digested_hex_file_name}");

    let destination = &("server/"
        .to_string()
        .add(&port.to_string())
        .add("/")
        .add(&digested_hex_file_name));
    let path = Path::new(destination);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(destination)?;
    file.write_all(&data)?;
    config.saved_files.insert(digested_hex_file_name.clone(), name);

    trace!("File stored successfully");
    Ok(digested_hex_file_name)
}