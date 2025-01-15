use std::net::{IpAddr, SocketAddr};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use DHTchord::node_state::NodeState;

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt()
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::NEW | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
        )
        .with_target(false)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // let (sender_put, receiver_put) = oneshot::channel();
    // let (sender_get, receiver_get) = oneshot::channel::<Result<common::File, ()>>();
    //
    //
    // tokio::spawn(async move {
    //     println!("{:?}", receiver_put.await.unwrap());
    //
    //     println!("{:?}", receiver_get.await.unwrap());
    // });

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
            sleep(Duration::from_secs(1));
            //
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8910".parse().unwrap()) {
                Ok(server2) => {
                    sleep(Duration::from_secs(2));
                    let span = tracing::trace_span!("127.0.0.1:8910");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server2.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_secs(2));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "7779".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:7779");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });

        scope.spawn(|| {
            sleep(Duration::from_secs(3));
            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "7778".parse().unwrap()) {
                Ok(server4) => {
                    let span = tracing::trace_span!("127.0.0.1:7778");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8910);
                    span.in_scope(|| server4.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
    });
}
