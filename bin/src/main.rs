use std::thread::sleep;
use std::time::Duration;
use DHTchord::node_state::NodeState;
use std::net::IpAddr;
use std::thread;

pub fn main(){
    let x= NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8888".parse().unwrap());
    let y= NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8910".parse().unwrap());


    thread::spawn(move || {
        x.run();
        println!("ciao");
        thread::sleep(Duration::from_secs(5));
    });
    println!("Hello, world!");
    sleep(Duration::from_secs(2));
    y.run();


    //sleep(Duration::new(1000, 10));

}