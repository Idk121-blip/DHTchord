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
            //
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

        scope.spawn(|| {
            //
            sleep(Duration::from_secs(4));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8912".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8912");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });

        scope.spawn(|| {
            //
            sleep(Duration::from_secs(5));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8913".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8913");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_secs(5));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8914".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8914");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_millis(5500));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8915".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8915");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_millis(6000));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8916".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8916");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            sleep(Duration::from_millis(7000));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8917".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8917");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            sleep(Duration::from_millis(7500));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8918".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8918");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            sleep(Duration::from_millis(8000));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8919".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8919");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_millis(8500));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "8920".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:8920");
                    let socket_addr = SocketAddr::new(IpAddr::V4("127.0.0.1".parse().unwrap()), 8911);
                    span.in_scope(|| server3.connect_and_run(socket_addr));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
    });
}
