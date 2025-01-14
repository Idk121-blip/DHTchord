use crate::common::UserMessage::{Get, Put};
use crate::common::{File, Message, ServerToUserMessage};
use message_io::network::{NetEvent, Transport};
use message_io::node;
use message_io::node::{NodeHandler, NodeListener};
use oneshot::Sender;
use std::io;
use std::net::SocketAddr;
use tracing::trace;

pub struct User {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
    listening_addr: SocketAddr,
}

impl User {
    pub fn new(ip_addr: String, port: String) -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let (_, listen_socket) = handler.network().listen(Transport::Ws, ip_addr + ":" + &port)?;
        let listening_addr = listen_socket;

        Ok(Self {
            handler,
            listener,
            listening_addr,
        })
    }

    pub fn put(self, server_address: &str, sender: Sender<Result<String, ()>>, file: File) {
        let Self {
            handler,
            listener,
            listening_addr,
        } = self;
        let (endpoint, _) = handler.network().connect_sync(Transport::Ws, server_address).unwrap();

        let message = Message::UserMessage(Put(file, listening_addr));
        let serialized = bincode::serialize(&message).unwrap();
        handler.network().send(endpoint, &serialized);

        let mut response = Err(()); // FIXME: () does not convey any meaning!

        listener.for_each(|event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();
                match server_to_user_message {
                    ServerToUserMessage::SavedKey(key) => {
                        trace!("Ok response from server, stopping myself");
                        response = Ok(key);
                        handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("forwarder");
                        //todo extend eventually a timer of the request
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Error returned from serve");
                        response = Err(()); // FIXME: () does not convey any meaning!
                        handler.stop();
                    }
                    other => panic!("received unexpected message: {:?}", other),
                }
            }
            NetEvent::Disconnected(_) => {}
        });

        sender.send(response).unwrap();
    }

    pub fn get(self, server_address: &str, sender: Sender<Result<File, ()>>, key: String) {
        let Self {
            handler,
            listener,
            listening_addr,
        } = self;
        let (endpoint, _) = handler.network().connect_sync(Transport::Ws, server_address).unwrap();

        let message = Message::UserMessage(Get(key, listening_addr));
        let serialized = bincode::serialize(&message).unwrap();
        handler.network().send(endpoint, &serialized);

        let mut response = Err(()); // FIXME: () does not convey any meaning!

        listener.for_each(|event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                // processor.sender_option.unwrap().send("message received".to_string());
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();

                match server_to_user_message {
                    ServerToUserMessage::RequestedFile(file) => {
                        trace!("File received");
                        response = Ok(file);
                        handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("Forwarded")
                    }
                    ServerToUserMessage::FileNotFound(_hex) => {
                        trace!("Not found");

                        response = Err(()); // FIXME: () does not convey any meaning!
                        handler.stop();
                    }
                    ServerToUserMessage::HexConversionNotValid(_) => {
                        trace!("hex conversion error");
                        response = Err(()); // FIXME: () does not convey any meaning!
                        handler.stop();
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Internal error while saving file");
                        response = Err(()); // FIXME: () does not convey any meaning!
                        handler.stop();
                    }
                    other => panic!("received unexpected message: {:?}", other),
                }
            }
            NetEvent::Disconnected(_) => {}
        });

        sender.send(response).unwrap();
    }
}
