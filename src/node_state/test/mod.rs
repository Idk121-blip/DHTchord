#[cfg(test)]
mod tests {
    use crate::common::{ChordMessage, File, Message, ServerSignals, UserMessage, SERVER_FOLDER};
    use crate::errors::GetError;
    use crate::node_state::handlers::server_message::join::handle_join;
    use crate::node_state::handlers::user_message::get::get_from_key;
    use crate::node_state::handlers::user_message::put::put_user_file;
    use crate::node_state::{NodeConfig, NodeState};
    use crate::user::User;
    use digest::Digest;
    use message_io::network::{NetEvent, SendStatus, Transport};
    use message_io::node::{NodeEvent, NodeHandler, NodeListener};
    use sha2::Sha256;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::ops::Add;
    use std::sync::mpsc::{Receiver, Sender};
    use std::{fs, thread};
    const LOCAL_IP_STR: &str = "127.0.0.1";
    const LOCAL_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

    // Helper function to create a test node
    fn create_test_node(port: u16) -> NodeState {
        NodeState::new(IpAddr::from(LOCAL_IP), port).unwrap_or_else(|_| panic!())
    }

    fn create_test_node_and_join(port: u16, port_into_join: u16) -> NodeState {
        let ip = IpAddr::from(LOCAL_IP);
        let socket_to_join = SocketAddr::new(ip, port_into_join);

        let node = create_test_node(port);

        let message = Message::ChordMessage(ChordMessage::Join(node.config.self_addr));

        let serialized = bincode::serialize(&message).unwrap();

        let (endpoint, _) = node.handler.network().connect(Transport::Ws, socket_to_join).unwrap();

        while node.handler.network().send(endpoint, &serialized) == SendStatus::ResourceNotAvailable {}
        node
    }

    fn bootstrap_node(
        tx: Sender<()>,
        handler_into_join: &NodeHandler<ServerSignals>,
        listener_into_join: NodeListener<ServerSignals>,
        mut config_into_join: &mut NodeConfig,
    ) {
        let mut join_counter = 0;
        tx.send(()).unwrap();
        listener_into_join.for_each(|event| {
            if let NodeEvent::Network(NetEvent::Message(endpoint, serialized)) = event {
                let message = bincode::deserialize(serialized).unwrap();
                if let Message::ChordMessage(ChordMessage::Join(new_node_address)) = message {
                    handle_join(handler_into_join, config_into_join, endpoint, new_node_address);
                    join_counter += 1;
                    if join_counter > 1 {
                        assert_ne!(
                            config_into_join.finger_table.first().unwrap().port(),
                            config_into_join.predecessor.unwrap().port()
                        );
                    }
                    if join_counter > 2 {
                        assert_ne!(
                            config_into_join.finger_table.first().unwrap().port(),
                            config_into_join.predecessor.unwrap().port()
                        );
                        handler_into_join.stop();
                    }
                    assert!(!config_into_join.finger_table.is_empty());
                    assert!(config_into_join.predecessor.is_some());
                }
            }
        });
        assert!(!config_into_join.finger_table.is_empty());
        assert!(config_into_join.predecessor.is_some());
        assert_ne!(
            config_into_join.finger_table.first().unwrap().port(),
            config_into_join.predecessor.unwrap().port()
        );
    }

    #[test]
    fn test_node_join() {
        const PORT_TO_JOIN: u16 = 8101;
        let node_into_join = create_test_node(PORT_TO_JOIN);

        let (tx, rx) = std::sync::mpsc::channel();
        let NodeState {
            handler: handler_into_join,
            listener: listener_into_join,
            config: mut config_into_join,
        } = node_into_join;
        assert!(config_into_join.finger_table.is_empty());
        assert!(config_into_join.predecessor.is_none());
        let handle_into_join = thread::spawn(move || {
            bootstrap_node(tx, &handler_into_join, listener_into_join, &mut config_into_join);
        });
        let handle_join = thread::spawn(move || {
            joining_nodes(rx, PORT_TO_JOIN);
        });
        handle_join.join().unwrap();
        handle_into_join.join().unwrap();
    }

    fn joining_nodes(rx: Receiver<()>, port_to_join: u16) {
        const FIRST_PORT_JOINING: u16 = 8091;
        const SECOND_PORT_JOINING: u16 = 8081;
        const THIRD_PORT_JOINING: u16 = 8071;
        rx.recv().unwrap();
        let joining_node_1 = create_test_node_and_join(FIRST_PORT_JOINING, port_to_join);
        joining_node_1.handler.stop();
        let joining_node_2 = create_test_node_and_join(SECOND_PORT_JOINING, port_to_join);
        joining_node_2.handler.stop();
        let joining_node_3 = create_test_node_and_join(THIRD_PORT_JOINING, port_to_join);
        joining_node_3.listener.for_each(|event| {
            if let NodeEvent::Network(NetEvent::Message(_, serialized)) = event {
                let message = bincode::deserialize(serialized).unwrap();
                if let Message::ChordMessage(ChordMessage::ForwardJoin(forward_address)) = message {
                    assert_ne!(forward_address.port(), port_to_join);
                    joining_node_3.handler.stop();
                }
            }
        });
    }

    #[test]
    fn test_user_put() {
        const PORT_TO_SAVE: u16 = 9001;
        const USER_PORT: u16 = 9000;
        let path = &(SERVER_FOLDER.to_string().add(&PORT_TO_SAVE.to_string()));
        let file_path = std::path::Path::new(path);
        let _ = fs::remove_dir(file_path);

        let node_into_join = create_test_node(PORT_TO_SAVE);
        let (tx, rx) = std::sync::mpsc::channel();
        let NodeState {
            handler: handler_into_join,
            listener: listener_into_join,
            config: mut config_into_join,
        } = node_into_join;
        let handle_into_join = thread::spawn(move || {
            tx.send(()).unwrap();
            listener_into_join.for_each(|event| {
                if let NodeEvent::Network(NetEvent::Message(endpoint, serialized)) = event {
                    let message = bincode::deserialize(serialized).unwrap();
                    if let Message::UserMessage(UserMessage::Put(file, user_address)) = message {
                        let digested_hex_file_name = hex::encode(Sha256::digest(file.name.as_bytes()));
                        let path = &(SERVER_FOLDER
                            .to_string()
                            .add(&PORT_TO_SAVE.to_string())
                            .add("/")
                            .add(&digested_hex_file_name));
                        let file_path = std::path::Path::new(path);
                        assert!(!(file_path.exists() && file_path.is_file()));

                        let server_to_user =
                            put_user_file(&handler_into_join, &mut config_into_join, file, user_address);
                        let serialized = bincode::serialize(&server_to_user).unwrap();
                        handler_into_join.network().send(endpoint, &serialized);

                        let file_path = std::path::Path::new(path);
                        assert!(file_path.exists() && file_path.is_file());
                        let _ = fs::remove_file(file_path);
                        handler_into_join.stop();
                    }
                }
            });
        });
        let handle_put = thread::spawn(move || {
            rx.recv().unwrap();
            let test_user = User::new(LOCAL_IP_STR.to_string(), USER_PORT.to_string());

            let file = File {
                name: "file_name".to_string(),
                buffer: vec![],
            };
            let key = test_user.unwrap().put(
                LOCAL_IP_STR
                    .to_string()
                    .add(":")
                    .add(PORT_TO_SAVE.to_string().as_str())
                    .as_str(),
                file,
            );
            assert!(key.is_ok());
            assert_ne!(key.unwrap().len(), 0);
        });

        handle_put.join().unwrap();
        handle_into_join.join().unwrap();
    }
    #[test]
    fn test_user_get() {
        const PORT_TO_SAVE: u16 = 9003;
        const USER_PORT: u16 = 9002;

        let node_into_join = create_test_node(PORT_TO_SAVE);
        let (tx, rx) = std::sync::mpsc::channel();
        let NodeState {
            handler: handler_into_join,
            listener: listener_into_join,
            config: mut config_into_join,
        } = node_into_join;
        thread::spawn(move || {
            tx.send(()).unwrap();
            listener_into_join.for_each(|event| {
                if let NodeEvent::Network(NetEvent::Message(endpoint, serialized)) = event {
                    let message = bincode::deserialize(serialized).unwrap();
                    if let Message::UserMessage(UserMessage::Put(file, user_address)) = message {
                        let server_to_user =
                            put_user_file(&handler_into_join, &mut config_into_join, file, user_address);
                        let serialized = bincode::serialize(&server_to_user).unwrap();
                        handler_into_join.network().send(endpoint, &serialized);
                    } else if let Message::UserMessage(UserMessage::Get(key, user_address)) = message {
                        let server_to_user = get_from_key(&handler_into_join, &mut config_into_join, user_address, key);
                        let serialized = bincode::serialize(&server_to_user).unwrap();
                        handler_into_join.network().send(endpoint, &serialized);
                    }
                }
            });
        });
        let handle_get = thread::spawn(move || {
            rx.recv().unwrap();
            let inserting_user = User::new(LOCAL_IP_STR.to_string(), USER_PORT.to_string());
            let file = File {
                name: "file_name".to_string(),
                buffer: vec![],
            };

            let result = inserting_user.unwrap().put(
                LOCAL_IP_STR
                    .to_string()
                    .add(":")
                    .add(PORT_TO_SAVE.to_string().as_str())
                    .as_str(),
                file,
            );
            assert!(result.is_ok());
            let key = result.unwrap();
            existing_key(key, PORT_TO_SAVE, USER_PORT);

            get_error_test();
        });

        handle_get.join().unwrap();
    }

    fn get_error_test() {
        const PORT_TO_SAVE: u16 = 9003;
        const USER_PORT: u16 = 9002;
        let not_existing_key = "nothexkey".to_string();
        let getting_user = User::new(LOCAL_IP_STR.to_string(), USER_PORT.to_string());
        let result = getting_user.unwrap().get(
            LOCAL_IP_STR
                .to_string()
                .add(":")
                .add(PORT_TO_SAVE.to_string().as_str())
                .as_str(),
            not_existing_key,
        );
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), GetError::HexConversion);

        let not_present_key = "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2".to_string();
        let getting_user = User::new(LOCAL_IP_STR.to_string(), USER_PORT.to_string());
        let result = getting_user.unwrap().get(
            LOCAL_IP_STR
                .to_string()
                .add(":")
                .add(PORT_TO_SAVE.to_string().as_str())
                .as_str(),
            not_present_key,
        );
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), GetError::NotFound);
    }

    fn existing_key(existing_key: String, port_to_save: u16, user_port: u16) {
        let getting_user = User::new(LOCAL_IP_STR.to_string(), user_port.to_string());
        let result = getting_user.unwrap().get(
            LOCAL_IP_STR
                .to_string()
                .add(":")
                .add(port_to_save.to_string().as_str())
                .as_str(),
            existing_key,
        );
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap().name, "file_name".to_string());
        assert_eq!(result.unwrap().buffer, vec![]);
    }
}
