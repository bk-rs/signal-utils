/*
RUST_BACKTRACE=1 RUST_LOG=trace cargo run -p signal-handler --example tokio --
*/

use std::{
    error,
    io::Error as IoError,
    process,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};

//
#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let port = portpicker::pick_unused_port().expect("No ports free");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    let (tcp_accept_tx, mut tcp_accept_rx) = mpsc::unbounded_channel();
    let tcp_accept_join_handle: JoinHandle<Result<(), IoError>> = tokio::spawn(async move {
        tcp_accept_rx.recv().await.unwrap();

        println!("telnet 127.0.0.1 {}", port);

        loop {
            let (mut stream, addr) = listener.accept().await?;
            println!("new client, addr:{:?}", addr);
            tokio::spawn(async move {
                let (mut r, mut w) = stream.split();
                match tokio::io::copy(&mut r, &mut w).await {
                    Ok(n) => {
                        println!("copy succeeded, addr:{:?} n:{}", addr, n);
                    }
                    Err(err) => {
                        println!("copy failed, addr:{:?} err:{}", addr, err);
                    }
                }
            });
        }

        #[allow(unreachable_code)]
        Ok(())
    });

    //
    let ctx = Arc::new(AtomicUsize::new(1));
    let instant = Instant::now();

    let handler = signal_handler::Handler::builder()
        .initialized(move |info| {
            println!("initialized info:{:?}", info);
            tcp_accept_tx.send(()).unwrap();

            let pid = process::id();
            println!("kill -HUP {}", pid);
            println!("kill -USR1 {}", pid);
            println!("kill -TERM {}", pid);
            println!("Control-C");
        })
        .reload_config({
            let ctx = ctx.clone();
            move |info| {
                ctx.fetch_add(1, Ordering::SeqCst);
                println!("reload_config info:{:?}", info);
            }
        })
        .print_stats({
            let ctx = ctx.clone();
            move |info| {
                println!(
                    "print_stats info:{:?} ctx:{:?} uptime:{:?}",
                    info,
                    ctx,
                    instant.elapsed(),
                );
            }
        })
        .wait_for_stop(|info| {
            println!("wait_for_stop info:{:?}", info);
        })
        .build();

    handler.handle_async().await?;

    //
    tcp_accept_join_handle.abort();
    assert!(tcp_accept_join_handle.await.unwrap_err().is_cancelled());

    Ok(())
}
