use std::io::Error;
use std::net::IpAddr;
use std::thread::sleep;
use std::time::Duration;
use std::{io, thread};
use DHTchord::node_state::NodeState;

pub fn main() {
    thread::scope(|scope| {
        scope.spawn(|| {
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8911".parse().unwrap()) {
                Ok(server1) => {
                    server1.run();
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8910".parse().unwrap()) {
                Ok(server2) => {
                    server2.run();
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "9000".parse().unwrap()) {
                Ok(server3) => {
                    server3.run();
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
    });
}
