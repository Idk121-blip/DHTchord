use crate::common::UserMessage::{Get, Put};
use crate::common::{File, Message, ServerToUserMessage};
use message_io::network::{NetEvent, Transport};
use message_io::node;
use message_io::node::{NodeHandler, NodeListener};
use oneshot::Sender;
use std::io;
use std::sync::{Arc, Mutex};
use tracing::trace;

pub struct User {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
}

impl User {
    pub fn new() -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let (_id, listen_address) = handler.network().listen(Transport::Ws, "0.0.0.0:0")?;
        trace!("{listen_address}");
        Ok(Self { handler, listener })
    }

    pub fn put(self, server_address: &str, sender: Sender<Result<String, ()>>, file: File) {
        let Self {
            handler,
            listener,
        } = self;
        let (ep, _) = handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();
        handler
            .network()
            .send(ep, &bincode::serialize(&Message::UserMessage(Put(file))).unwrap());

        let response = Arc::new(Mutex::new(Err(())));

        let response_clone = Arc::clone(&response);

        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                trace!("response from server, killing myself");
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();
                match server_to_user_message {
                    ServerToUserMessage::SavedKey(key) => {
                        *response_clone.lock().unwrap() = Ok(key);
                        handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        //todo extend eventually a timer of the request
                    }
                    _ => {}
                }
            }
            NetEvent::Disconnected(_) => {}
        });
        let final_response = response.lock().unwrap().clone();
        sender.send(final_response).unwrap();
    }

    pub fn get(self, server_address: &str, sender: Sender<Option<File>>, key: String) {
        let Self {
            handler,
            listener,
        } = self;
        let (ep, _) = handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();
        handler
            .network()
            .send(ep, &bincode::serialize(&Message::UserMessage(Get(key))).unwrap());

        let response = Arc::new(Mutex::new(None));

        let response_clone = Arc::clone(&response);

        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                trace!("response from server, killing myself");
                // processor.sender_option.unwrap().send("message received".to_string());
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();
                if let ServerToUserMessage::RequestedFile(file) = server_to_user_message {
                    trace!("Server message");
                    *response_clone.lock().unwrap() = Some(file);
                }
                handler.stop();
            }
            NetEvent::Disconnected(_) => {}
        });


        let final_response = response.lock().unwrap().take();
        sender.send(final_response).unwrap();
    }
}