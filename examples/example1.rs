use std::net::IpAddr;
use std::thread::sleep;
use std::time::Duration;
use std::{io, thread};
use DHTchord::node_state::NodeState;

pub fn main() -> Result<(), io::Error> {
    let z = NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8911".parse().unwrap())?;
    thread::spawn(move || {
        z.run();
    });

    sleep(Duration::from_secs(2));
    let y = NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8910".parse().unwrap())?;
    thread::spawn(move || {
        y.run();
    });

    sleep(Duration::from_secs(2));
    let x = NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "9000".parse().unwrap())?;

    x.run();
    //sleep(Duration::new(1000, 10));

    Ok(())
}
