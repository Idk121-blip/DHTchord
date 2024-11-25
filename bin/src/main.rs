use std::thread::sleep;
use std::time::Duration;
use DHTchord::node_state::NodeState;

pub fn main(){
    let x= NodeState::new();

    x.run();

    sleep(Duration::new(1000, 10));

}