use std::fs::File;
use std::io::Read;
use std::net::IpAddr;
use std::ops::Add;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use DHTchord::common;
use DHTchord::node_state::NodeState;
use DHTchord::user::User;

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
                    span.in_scope(|| server2.connect_and_run("127.0.0.1:8911"));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        scope.spawn(|| {
            //
            sleep(Duration::from_secs(2));

            match NodeState::new(IpAddr::V4("127.0.0.1".parse().unwrap()), "7777".parse().unwrap()) {
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:7777");
                    span.in_scope(|| server3.connect_and_run("127.0.0.1:8911"));
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
                Ok(server3) => {
                    let span = tracing::trace_span!("127.0.0.1:7778");
                    span.in_scope(|| server3.connect_and_run("127.0.0.1:8910"));
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }
        });
        // scope.spawn(|| {
        //     sleep(Duration::from_secs(5));
        //
        //     match User::new("127.0.0.1".to_string(), "8700".to_string()) {
        //         Ok(user1) => {
        //             sleep(Duration::from_secs(5));
        //             let span = tracing::trace_span!("User1");
        //
        //             span.in_scope(|| {
        //                 let file_name = "prova2.txt";
        //                 let file_path = "user/".to_string().add(file_name);
        //                 let file = File::open(file_path);
        //
        //                 let mut buffer = Vec::new();
        //
        //                 let _ = file.unwrap().read_to_end(&mut buffer);
        //
        //                 let file = common::File {
        //                     name: file_name.to_string(),
        //                     buffer,
        //                 };
        //
        //                 user1.put("127.0.0.1:7777", sender_put, file);
        //             });
        //         }
        //         Err(error) => {
        //             eprintln!("{:?}", error)
        //         }
        //     }
        // });
        //
        // scope.spawn(|| {
        //     sleep(Duration::from_secs(5));
        //
        //     match User::new("127.0.0.1".to_string(), "8800".to_string()) {
        //         Ok(user2) => {
        //             let span = tracing::trace_span!("User2");
        //             span.in_scope(|| {
        //                 sleep(Duration::from_secs(10));
        //                 let input = "ac9694c9206dd5a9e51e956a07ade297dd9b4a65ff146629aa6cb5aa08eaacd0".to_string();
        //                 user2.get("127.0.0.1:7777", sender_get, input);
        //             });
        //         }
        //         Err(error) => {
        //             eprintln!("{:?}", error)
        //         }
        //     }
        // });
    });
}
