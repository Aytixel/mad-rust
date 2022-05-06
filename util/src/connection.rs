pub use server::Server;

mod server {
    use std::io::prelude::*;
    use std::net::TcpListener;
    use std::sync::mpsc::TryRecvError;
    use std::thread::spawn;
    use std::time::{Duration, Instant};

    use crate::thread::DualChannel;
    use crate::time::{Timer, TIMEOUT_1S};

    pub struct Server {
        pub dual_channel: DualChannel<Vec<u8>>,
    }

    impl Server {
        pub fn new() -> Self {
            let (host, child) = DualChannel::<Vec<u8>>::new();

            spawn(move || {
                let mut timer = Timer::new(TIMEOUT_1S);

                loop {
                    if let Ok(listener) = TcpListener::bind("127.0.0.1:651") {
                        for mut socket in listener.incoming().filter_map(|x| x.ok()) {
                            if let Ok(_) = socket.set_nonblocking(true) {
                                let mut child = child.clone();

                                spawn(move || {
                                    // data communication handling
                                    let mut timer = Timer::new(Duration::from_millis(200));
                                    let mut size_buffer = [0; 8];
                                    let mut last_packet_send = Instant::now();
                                    let mut last_packet_receive = Instant::now();

                                    'main: loop {
                                        if last_packet_receive.elapsed() > Duration::from_secs(10) {
                                            break;
                                        }

                                        if last_packet_send.elapsed() > Duration::from_secs(1) {
                                            socket.write_all(&u64::MAX.to_be_bytes()).ok();

                                            last_packet_send = Instant::now();
                                        }

                                        // data from the client
                                        if let Ok(_) = socket.read_exact(&mut size_buffer) {
                                            let size = u64::from_be_bytes(size_buffer);

                                            // connection end
                                            if size == 0 {
                                                break;
                                            }

                                            // life packet
                                            if size == u64::MAX {
                                                last_packet_receive = Instant::now();
                                            } else {
                                                let mut buffer = vec![0; size as usize];

                                                if let Ok(_) = socket.read_exact(&mut buffer) {
                                                    child.send(buffer).ok();
                                                }
                                            }
                                        }

                                        // data to the client
                                        loop {
                                            match child.try_recv() {
                                                Ok(data) => {
                                                    socket
                                                        .write_all(
                                                            &(data.len() as u64).to_be_bytes(),
                                                        )
                                                        .ok();
                                                    socket.write_all(&data).ok();
                                                }
                                                Err(TryRecvError::Disconnected) => {
                                                    socket.write_all(&0u64.to_be_bytes()).ok();
                                                    break 'main;
                                                }
                                                _ => break,
                                            }
                                        }

                                        timer.wait();
                                    }
                                });
                            }
                        }
                    }

                    timer.wait();
                }
            });

            Self { dual_channel: host }
        }
    }
}

pub use client::Client;

mod client {
    use std::io::prelude::*;
    use std::net::TcpStream;
    use std::sync::mpsc::TryRecvError;
    use std::thread::spawn;
    use std::time::{Duration, Instant};

    use crate::thread::DualChannel;
    use crate::time::{Timer, TIMEOUT_1S};

    pub struct Client {
        pub dual_channel: DualChannel<Vec<u8>>,
    }

    impl Client {
        pub fn new(connection_start_data: Vec<u8>) -> Self {
            let (host, mut child) = DualChannel::<Vec<u8>>::new();

            spawn(move || {
                let mut timer = Timer::new(TIMEOUT_1S);

                loop {
                    if let Ok(mut socket) = TcpStream::connect("127.0.0.1:651") {
                        if let Ok(_) = socket.set_nonblocking(true) {
                            // data communication handling
                            let mut timer = Timer::new(Duration::from_millis(100));
                            let mut size_buffer = [0; 8];
                            let mut last_packet_send = Instant::now();
                            let mut last_packet_receive = Instant::now();

                            socket
                                .write_all(&(connection_start_data.len() as u64).to_be_bytes())
                                .ok();
                            socket.write_all(&connection_start_data).ok();

                            'main: loop {
                                if last_packet_receive.elapsed() > Duration::from_secs(10) {
                                    break;
                                }

                                if last_packet_send.elapsed() > Duration::from_secs(1) {
                                    socket.write_all(&u64::MAX.to_be_bytes()).ok();

                                    last_packet_send = Instant::now();
                                }

                                // data from the server
                                if let Ok(_) = socket.read_exact(&mut size_buffer) {
                                    let size = u64::from_be_bytes(size_buffer);

                                    // connection end
                                    if size == 0 {
                                        break;
                                    }

                                    // life packet
                                    if size == u64::MAX {
                                        last_packet_receive = Instant::now();
                                    } else {
                                        let mut buffer = vec![0; size as usize];

                                        if let Ok(_) = socket.read_exact(&mut buffer) {
                                            child.send(buffer).ok();
                                        }
                                    }
                                }

                                // data to the server
                                loop {
                                    match child.try_recv() {
                                        Ok(data) => {
                                            socket
                                                .write_all(&(data.len() as u64).to_be_bytes())
                                                .ok();
                                            socket.write_all(&data).ok();

                                            last_packet_send = Instant::now();
                                        }
                                        Err(TryRecvError::Disconnected) => {
                                            socket.write_all(&0u64.to_be_bytes()).ok();
                                            break 'main;
                                        }
                                        _ => break,
                                    }
                                }

                                timer.wait();
                            }
                        }
                    }

                    timer.wait();
                }
            });

            Self { dual_channel: host }
        }
    }
}
