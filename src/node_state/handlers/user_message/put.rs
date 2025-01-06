use crate::common;
use crate::common::{
    binary_search, get_endpoint, ChordMessage, Message, ServerSignals, ServerToUserMessage, SERVER_FOLDER,
};
use crate::errors::PutError;
use crate::node_state::{NodeConfig, SAVED_FILES};
use digest::Digest;
use message_io::node::NodeHandler;
use sha2::Sha256;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::ops::Add;
use std::path::Path;
use std::{fs, io};
use tracing::trace;

pub fn handle_forwarded_put(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    addr: SocketAddr,
    file: common::File,
) {
    let endpoint = get_endpoint(handler, config, addr);
    let message = ServerSignals::SendMessageToUser(endpoint, put_user_file(handler, config, file, addr));
    handler.signals().send(message);
}

pub fn put_user_file(
    handler: &NodeHandler<ServerSignals>,
    config: &mut NodeConfig,
    file: common::File,
    user_addr: SocketAddr,
) -> ServerToUserMessage {
    match handle_user_put(handler, file, config, user_addr) {
        Ok(saved_key) => ServerToUserMessage::SavedKey(saved_key),
        Err(error) => match error {
            PutError::ForwardingRequest(address) => ServerToUserMessage::ForwarderTo(address),
            PutError::ErrorStoringFile => ServerToUserMessage::InternalServerError,
        },
    }
}

fn handle_user_put(
    handler: &NodeHandler<ServerSignals>,
    file: common::File,
    config: &mut NodeConfig,
    addr: SocketAddr,
) -> Result<String, PutError> {
    let digested_file_name = Sha256::digest(file.name.as_bytes()).to_vec();
    let successor = Sha256::digest(config.finger_table[0].to_string().as_bytes()).to_vec();

    if (digested_file_name > config.id && (digested_file_name < successor || config.id > successor))
        || config.id > successor && digested_file_name < successor
        || config.finger_table.is_empty()
    {
        return save_in_server(file, config.self_addr.port(), config).map_or(Err(PutError::ErrorStoringFile), Ok);
    }

    let forwarding_index = binary_search(config, &digested_file_name);

    let forwarding_endpoint = get_endpoint(handler, config, addr);

    handler.signals().send(ServerSignals::ForwardMessage(
        forwarding_endpoint,
        Message::ChordMessage(ChordMessage::ForwardedPut(addr, file)),
    ));

    Err(PutError::ForwardingRequest(
        config.finger_table[forwarding_index].to_string(),
    ))
}

pub fn save_in_server(file: common::File, port: u16, config: &mut NodeConfig) -> io::Result<String> {
    let common::File { name, buffer: data } = file;

    let digested_hex_file_name = hex::encode(Sha256::digest(name.as_bytes()));

    let destination = &(SERVER_FOLDER
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
    file.flush()?;

    if !config.saved_files.contains_key(&digested_hex_file_name) {
        append_in_saved_files_file(port, digested_hex_file_name.clone(), name.clone())?;
    }
    config.saved_files.insert(digested_hex_file_name.clone(), name);
    trace!("File stored successfully");
    Ok(digested_hex_file_name)
}

fn append_in_saved_files_file(port: u16, digested_hex_file: String, file_name: String) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(SERVER_FOLDER.to_string() + port.to_string().as_str() + "/" + SAVED_FILES)?;

    writeln!(file, "{}", digested_hex_file + ":" + file_name.as_str())?;
    Ok(())
}
