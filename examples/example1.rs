use std::net::IpAddr;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use DHTchord::node_state::NodeState;
use DHTchord::user::User;

pub fn main() {
    tracing_subscriber::fmt()
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::NEW | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
        )
        .with_target(false)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    thread::scope(|scope| {
        scope.spawn(|| {
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8911".parse().unwrap()) {
                Ok(server1) => {
                    let span = tracing::trace_span!("127.0.0.1:8911");
                    span.in_scope(|| server1.run());
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
                    sleep(Duration::from_secs(2));
                    let span = tracing::trace_span!("127.0.0.1:8910");
                    span.in_scope(|| server2.connect_to("127.0.0.1:8911").unwrap().run());
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "7777".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:7777");
                    span.in_scope(|| server3.connect_to("127.0.0.1:8911").unwrap().run());
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            match User::new() {
                Ok(user1) => {
                    sleep(Duration::from_secs(5));
                    let span = tracing::trace_span!("User1");
                    let (sender, receiver) = oneshot::channel();
                    span.in_scope(|| {
                        user1.put("127.0.0.1:7777", sender);
                        sleep(Duration::from_secs(8));
                        let mut input = "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2".to_string();

                        let (sender, receiver) = oneshot::channel();
                        let user1 = User::new().unwrap();
                        user1.get("127.0.0.1:7777", sender, input);
                    });
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
    });
}
