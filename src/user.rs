use crate::common::Message;
use crate::common::UserMessage::Put;
use message_io::network::{NetEvent, Transport};
use message_io::node;
use message_io::node::{NodeHandler, NodeListener};
use oneshot::Sender;
use std::io;
use std::marker::PhantomData;
use tracing::trace;

pub struct User<T> {
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
    phantom: PhantomData<T>,
}

impl<T> User<T> {
    pub fn new() -> Result<Self, io::Error> {
        let (handler, listener) = node::split();
        let (_id, listen_address) = handler.network().listen(Transport::Ws, "0.0.0.0:0")?;
        trace!("{listen_address}");
        Ok(Self { handler, listener, phantom: PhantomData })
    }

    pub fn put(self, server_address: &str, sender: Sender<String>, data: T) {
        let (ep, _) = self
            .handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();
        self.handler
            .network()
            .send(ep, &bincode::serialize(&Message::UserMessage(Put(12))).unwrap());
        self.listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, _) => {
                trace!("response from server, killing myself");
                // processor.sender_option.unwrap().send("message received".to_string());
                self.handler.stop();
            }
            NetEvent::Disconnected(_) => {}
        });
    }
}