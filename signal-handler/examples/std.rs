/*
RUST_BACKTRACE=1 RUST_LOG=trace cargo run -p signal-handler --example std --
*/

use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use std::{
    io::{Error as IoError, Read as _, Write as _},
    net::TcpListener,
    process,
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::Instant,
};

//
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = portpicker::pick_unused_port().expect("No ports free");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;

    let (tcp_accept_tx, tcp_accept_rx) = mpsc::sync_channel::<()>(1);
    let tcp_accept_join_handle: JoinHandle<Result<(), IoError>> = thread::spawn(move || {
        tcp_accept_rx.recv().unwrap();

        println!("telnet 127.0.0.1 {}", port);

        loop {
            let (mut stream, addr) = listener.accept()?;

            println!("new client, addr:{:?}", addr);
            thread::spawn(move || {
                let mut buf = vec![0; 1024];
                let mut n = 0;

                loop {
                    match stream.read(&mut buf[..]) {
                        Ok(n_tmp) => {
                            if n_tmp == 0 {
                                println!("copy succeeded, addr:{:?} n:{}", addr, n);
                                break;
                            }
                            match stream.write_all(&buf[..n_tmp]) {
                                Ok(_) => {
                                    n += n_tmp;
                                }
                                Err(err) => {
                                    println!("write failed, addr:{:?} err:{}", addr, err);
                                }
                            }
                        }
                        Err(err) => {
                            println!("read failed, addr:{:?} err:{}", addr, err);
                        }
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
                thread::sleep(Duration::from_secs(3));

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
            // e.g.
            // rocksdb::DBWithThreadMode::flush_wal

            println!("wait_for_stop info:{:?}", info);
        })
        .build();

    handler.handle()?;

    //
    drop(tcp_accept_join_handle);

    Ok(())
}
