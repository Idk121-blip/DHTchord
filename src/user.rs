use crate::common::UserMessage::{Get, Put};
use crate::common::{File, Message, ServerToUserMessage};
use crate::errors::GetError::{ErrorRetrievingFile, HexConversion, NotFound};
use crate::errors::PutError::ErrorStoringFile;
use crate::errors::{GetError, PutError};
use message_io::network::{NetEvent, Transport};
use message_io::node;
use message_io::node::{NodeHandler, NodeListener};
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

    /// Sends a file to a remote server, handles the server's response, and communicates the result back via a channel.
    ///
    ///
    /// # Parameters
    /// - `self`: Consumes the instance of the struct to perform the operation. This implies that the instance cannot be reused after this function call.
    /// - `server_address`: A string slice (`&str`) representing the address of the server to which the file will be sent.
    /// - `sender`: A channel sender (`Sender<Result<String, PutError>>`) used to send the result of the operation back to the caller.
    ///   - On success, the sender transmits an `Ok(String)` containing the key returned by the server.
    ///   - On failure, the sender transmits an `Err(PutError)`.
    /// - `file`: A `File` instance representing the file to be sent to the server.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::mpsc::channel;
    /// use crate::DHTchord::user::User;
    /// use crate::DHTchord::common::File;
    ///
    /// let (tx, rx) = oneshot::channel();
    /// let file = File{name: "".to_string(),buffer: vec![]};
    /// let instance = User::new("127.0.0.1".to_string(), "8700".to_string()).unwrap(); // Replace with the actual struct implementing this method.
    ///
    /// instance.put("127.0.0.1:7777", file);
    ///
    /// match rx.recv().unwrap() {
    ///     Ok(key) => println!("File stored successfully, key: {}", key),
    ///     Err(err) => println!("Failed to store file: {:?}", err),
    /// }
    /// ```

    pub fn put(self, server_address: &str, file: File) -> Result<String, PutError> {
        let (endpoint, _) = self
            .handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();

        let message = Message::UserMessage(Put(file, self.listening_addr));
        let serialized = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &serialized);

        let mut response = Err(ErrorStoringFile);

        self.listener.for_each(|event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();
                match server_to_user_message {
                    ServerToUserMessage::SavedKey(key) => {
                        trace!("Ok response from server, stopping myself");
                        response = Ok(key);
                        self.handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("forwarder");
                        //todo extend eventually a timer of the request
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Error returned from serve");
                        response = Err(ErrorStoringFile);
                        self.handler.stop();
                    }
                    other => panic!("received unexpected message: {:?}", other),
                }
            }
            NetEvent::Disconnected(_) => {}
        });

        response
    }

    /// Retrieves a file from a remote server using a key and communicates the result back via a channel.
    ///
    /// This function performs the following operations:
    /// 1. Establishes a connection with the server using a WebSocket transport.
    /// 2. Sends a request to the server to retrieve the file corresponding to the provided key.
    /// 3. Listens for events and processes the server's response to determine the outcome (success or failure).
    /// 4. Sends the result (the retrieved file or an error) back to the caller via the provided `Sender`.
    ///
    /// # Parameters
    /// - `self`: Consumes the instance of the struct to perform the operation. This implies that the instance cannot be reused after this function call.
    /// - `server_address`: A string slice (`&str`) representing the address of the server to which the file retrieval request is sent.
    /// - `sender`: A channel sender (`Sender<Result<File, GetError>>`) used to send the result of the operation back to the caller.
    ///   - On success, the sender transmits an `Ok(File)` containing the retrieved file.
    ///   - On failure, the sender transmits an `Err(GetError)`, specifying the type of error encountered.
    /// - `key`: A `String` representing the unique identifier (key) for the file to be retrieved.
    ///
    /// # Panics
    /// - The function will panic if:
    ///   - Serialization of the message fails.
    ///   - An unexpected server message is received.
    ///   - The `sender.send` call fails.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::mpsc::channel;
    /// use crate::DHTchord::user::User;
    ///
    /// let (tx, rx) = oneshot::channel();
    /// let instance = User::new("127.0.0.1".to_string(), "8700".to_string()).unwrap();
    ///
    /// instance.get("127.0.0.1:7777", "string_key".to_string());
    /// match rx.recv().unwrap() {
    ///     Ok(file) => println!("File retrieved successfully: {:?}", file),
    ///     Err(err) => println!("Failed to retrieve file: {:?}", err),
    /// }
    /// ```

    pub fn get(self, server_address: &str, key: String) -> Result<File, GetError> {
        let (endpoint, _) = self
            .handler
            .network()
            .connect_sync(Transport::Ws, server_address)
            .unwrap();

        let message = Message::UserMessage(Get(key, self.listening_addr));
        let serialized = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &serialized);

        let mut response = Err(ErrorRetrievingFile);

        self.listener.for_each(|event| match event.network() {
            NetEvent::Connected(_, _) => {}
            NetEvent::Accepted(_, _) => {}
            NetEvent::Message(_, bytes) => {
                // processor.sender_option.unwrap().send("message received".to_string());
                let server_to_user_message: ServerToUserMessage = bincode::deserialize(bytes).unwrap();

                match server_to_user_message {
                    ServerToUserMessage::RequestedFile(file) => {
                        trace!("File received");
                        response = Ok(file);
                        self.handler.stop();
                    }
                    ServerToUserMessage::ForwarderTo(_) => {
                        trace!("Forwarded")
                    }
                    ServerToUserMessage::FileNotFound(_hex) => {
                        trace!("Not found");

                        response = Err(NotFound);
                        self.handler.stop();
                    }
                    ServerToUserMessage::HexConversionNotValid(_) => {
                        trace!("hex conversion error");
                        response = Err(HexConversion);
                        self.handler.stop();
                    }
                    ServerToUserMessage::InternalServerError => {
                        trace!("Internal error while saving file");
                        response = Err(ErrorRetrievingFile);
                        self.handler.stop();
                    }
                    other => panic!("received unexpected message: {:?}", other),
                }
            }
            NetEvent::Disconnected(_) => {}
        });

        response
    }
}
