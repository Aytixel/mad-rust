pub use server::Server;

pub mod server {
    use std::io::prelude::*;
    use std::net::TcpListener;
    use std::thread::{current, spawn, ThreadId};
    use std::time::{Duration, Instant};

    use crate::thread::DualChannel;
    use crate::time::{Timer, TIMEOUT_1S};

    pub struct Server {
        pub dual_channel: DualChannel<(ThreadId, bool, Vec<u8>)>,
    }

    impl Server {
        pub fn new() -> Self {
            let (host, child) = DualChannel::<(ThreadId, bool, Vec<u8>)>::new();

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

                                    child.send((current().id(), true, vec![])).ok();

                                    'main: loop {
                                        if last_packet_receive.elapsed() > Duration::from_secs(10) {
                                            child.send((current().id(), false, vec![])).ok();
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
                                                child.send((current().id(), false, vec![])).ok();
                                                break;
                                            }

                                            // life packet
                                            last_packet_receive = Instant::now();

                                            if size != u64::MAX {
                                                let mut buffer = vec![0; size as usize];

                                                if let Ok(_) = socket.read_exact(&mut buffer) {
                                                    child.send((current().id(), true, buffer)).ok();
                                                }
                                            }
                                        }

                                        // data to the client
                                        loop {
                                            match child.try_recv() {
                                                Ok((thread_id, is_running, data)) => {
                                                    if !is_running {
                                                        socket.write_all(&0u64.to_be_bytes()).ok();
                                                        break 'main;
                                                    }

                                                    if thread_id == current().id() {
                                                        socket
                                                            .write_all(
                                                                &(data.len() as u64).to_be_bytes(),
                                                            )
                                                            .ok();
                                                        socket.write_all(&data).ok();
                                                    }
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

pub mod client {
    use std::io::prelude::*;
    use std::net::TcpStream;
    use std::thread::spawn;
    use std::time::{Duration, Instant};

    use crate::thread::DualChannel;
    use crate::time::{Timer, TIMEOUT_1S};

    pub struct Client {
        pub dual_channel: DualChannel<(bool, Vec<u8>)>,
    }

    impl Client {
        pub fn new(connection_start_data: Vec<u8>) -> Self {
            let (host, mut child) = DualChannel::<(bool, Vec<u8>)>::new();

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

                            child.send((true, vec![])).ok();

                            socket
                                .write_all(&(connection_start_data.len() as u64).to_be_bytes())
                                .ok();
                            socket.write_all(&connection_start_data).ok();

                            'main: loop {
                                if last_packet_receive.elapsed() > Duration::from_secs(10) {
                                    child.send((false, vec![])).ok();
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
                                        child.send((false, vec![])).ok();
                                        break;
                                    }

                                    // life packet
                                    last_packet_receive = Instant::now();

                                    if size != u64::MAX {
                                        let mut buffer = vec![0; size as usize];

                                        if let Ok(_) = socket.read_exact(&mut buffer) {
                                            child.send((true, buffer)).ok();
                                        }
                                    }
                                }

                                // data to the server
                                loop {
                                    match child.try_recv() {
                                        Ok((is_running, data)) => {
                                            if !is_running {
                                                socket.write_all(&0u64.to_be_bytes()).ok();
                                                break 'main;
                                            }

                                            socket
                                                .write_all(&(data.len() as u64).to_be_bytes())
                                                .ok();
                                            socket.write_all(&data).ok();

                                            last_packet_send = Instant::now();
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

pub use command::{Command, CommandTrait};

pub mod command {
    pub trait CommandTrait {
        fn to_bytes(&self) -> Vec<u8>;

        fn from_bytes(data: Vec<u8>) -> Self;
    }

    pub struct Command {
        pub data: Vec<u8>,
        pub id: u8,
    }

    impl Command {
        pub fn new(id: u8) -> Self {
            Self {
                data: vec![],
                id: id,
            }
        }

        pub fn add_bytes(&mut self, data: &mut Vec<u8>) -> &mut Self {
            self.add_u32(data.len() as u32);
            self.data.append(data);
            self
        }

        pub fn add_byte(&mut self, data: u8) -> &mut Self {
            self.data.push(data);
            self
        }

        pub fn add_string(&mut self, data: String) -> &mut Self {
            self.add_u32(data.len() as u32);
            self.data.append(&mut data.as_bytes().to_vec());
            self
        }

        pub fn add_u32(&mut self, data: u32) -> &mut Self {
            self.data.append(&mut data.to_be_bytes().to_vec());
            self
        }

        pub fn add_i32(&mut self, data: i32) -> &mut Self {
            self.data.append(&mut data.to_be_bytes().to_vec());
            self
        }

        pub fn get_bytes(&mut self) -> Vec<u8> {
            let number = self.get_u32() as usize;
            let data = self.data[..number].to_vec();

            self.data = self.data[number..].to_vec();

            data
        }

        pub fn get_byte(&mut self) -> u8 {
            let data: u8 = self.data[0];

            self.data = self.data[0..].to_vec();

            data
        }

        pub fn get_string(&mut self) -> String {
            let number = self.get_u32() as usize;
            let data = self.data[..number].to_vec();

            self.data = self.data[number..].to_vec();

            String::from_utf8(data).unwrap()
        }

        pub fn get_u32(&mut self) -> u32 {
            let data: [u8; 4] = self.data[..4].try_into().unwrap();

            self.data = self.data[4..].to_vec();

            u32::from_be_bytes(data)
        }

        pub fn get_i32(&mut self) -> i32 {
            let data: [u8; 4] = self.data[..4].try_into().unwrap();

            self.data = self.data[4..].to_vec();

            i32::from_be_bytes(data)
        }
    }

    impl CommandTrait for Command {
        fn to_bytes(&self) -> Vec<u8> {
            let mut data = vec![self.id];

            data.append(&mut self.data.clone());
            data
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            Self {
                data: data[1..].to_vec(),
                id: data[0],
            }
        }
    }
}
