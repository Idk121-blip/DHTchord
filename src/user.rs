use crate::common::UserMessage::{Get, Put};
use crate::common::{File, Message, ServerToUserMessage};
use message_io::network::{NetEvent, Transport};
use message_io::node;
use message_io::node::{NodeHandler, NodeListener};
use oneshot::Sender;
use std::io;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tracing::trace;

pub struct User {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
    listening_addr: String,
}

impl User {
    pub fn new(ip_addr: String, port: String) -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let (_id, listen_socket) = handler.network().listen(Transport::Ws, ip_addr + ":" + &port)?;
        let listening_addr = listen_socket.to_string();
        println!("{listening_addr}");
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
        let (ep, _) = handler.network().connect_sync(Transport::Ws, server_address).unwrap();
        handler.network().send(
            ep,
            &bincode::serialize(&Message::UserMessage(Put(file, listening_addr))).unwrap(),
        );

        let response = Arc::new(Mutex::new(Err(())));

        let response_clone = Arc::clone(&response);

        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();
                match server_to_user_message {
                    ServerToUserMessage::SavedKey(key) => {
                        trace!("Ok response from server, killing myself");
                        *response_clone.lock().unwrap() = Ok(key);
                        handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("forwarder");
                        //todo extend eventually a timer of the request
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Error returned from serve");
                        *response_clone.lock().unwrap() = Err(());
                        handler.stop();
                    }
                    _ => {
                        trace!("Shouldn't arrive here");
                    }
                }
            }
            NetEvent::Disconnected(_) => {}
        });
        let final_response = response.lock().unwrap().clone();
        sender.send(final_response).unwrap();
    }

    pub fn get(self, server_address: &str, sender: Sender<Result<File, ()>>, key: String) {
        let Self {
            handler,
            listener,
            listening_addr,
        } = self;
        let (ep, _) = handler.network().connect_sync(Transport::Ws, server_address).unwrap();
        handler.network().send(
            ep,
            &bincode::serialize(&Message::UserMessage(Get(key, listening_addr))).unwrap());

        let response = Arc::new(Mutex::new(Err(())));

        let response_clone = Arc::clone(&response);

        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                // processor.sender_option.unwrap().send("message received".to_string());
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();

                match server_to_user_message {
                    ServerToUserMessage::RequestedFile(file) => {
                        trace!("File received");
                        *response_clone.lock().unwrap() = Ok(file);
                        handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("Forwarded")
                    }
                    ServerToUserMessage::FileNotFound(hex) => {
                        trace!("Not found");

                        *response_clone.lock().unwrap() = Err(());
                        handler.stop();
                    }
                    ServerToUserMessage::HexConversionNotValid(_) => {
                        trace!("hex conversion error");
                        *response_clone.lock().unwrap() = Err(());
                        handler.stop();
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Internal error while saving file");
                        *response_clone.lock().unwrap() = Err(());
                        handler.stop();
                    }
                    _ => {}
                }
            }
            NetEvent::Disconnected(_) => {}
        });

        let final_response = response.lock().unwrap().clone();
        sender.send(final_response).unwrap();
    }
}
