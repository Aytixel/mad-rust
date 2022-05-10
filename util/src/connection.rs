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
        pub fn new() -> Self {
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
    use std::io::{Cursor, Read, Write};

    pub trait CommandTrait {
        fn to_bytes(&mut self) -> Vec<u8>;

        fn from_bytes(data: Vec<u8>) -> Self;
    }

    #[derive(Debug)]
    pub struct Command {
        pub data: Cursor<Vec<u8>>,
        pub id: u8,
    }

    impl Command {
        pub fn new(id: u8) -> Self {
            Self {
                data: Cursor::new(Vec::new()),
                id: id,
            }
        }

        pub fn add_bytes(&mut self, data: &mut Vec<u8>) -> &mut Self {
            self.add_u32(data.len() as u32);
            self.data.write(data).unwrap();
            self
        }

        pub fn add_byte(&mut self, data: u8) -> &mut Self {
            self.data.write(&[data]).unwrap();
            self
        }

        pub fn add_string(&mut self, data: String) -> &mut Self {
            self.add_u32(data.len() as u32);
            self.data.write(&mut data.as_bytes().to_vec()).unwrap();
            self
        }

        pub fn add_u32(&mut self, data: u32) -> &mut Self {
            self.data.write(&mut data.to_be_bytes().to_vec()).unwrap();
            self
        }

        pub fn get_bytes(&mut self) -> Vec<u8> {
            let mut data = vec![0u8; self.get_u32() as usize];

            self.data.read_exact(&mut data).unwrap();

            data
        }

        pub fn get_byte(&mut self) -> u8 {
            let mut data = [0u8; 1];

            self.data.read_exact(&mut data).unwrap();

            data[0]
        }

        pub fn get_string(&mut self) -> String {
            let mut data = vec![0u8; self.get_u32() as usize];

            self.data.read_exact(&mut data).unwrap();

            String::from_utf8(data).unwrap()
        }

        pub fn get_u32(&mut self) -> u32 {
            let mut data = [0u8; 4];

            self.data.read_exact(&mut data).unwrap();

            u32::from_be_bytes(data)
        }
    }

    impl CommandTrait for Command {
        fn to_bytes(&mut self) -> Vec<u8> {
            let mut data = vec![self.id];
            let mut cursor_data = vec![0u8; self.data.position() as usize];

            self.data.set_position(0);
            self.data.read_exact(&mut cursor_data).unwrap();

            data.append(&mut cursor_data);
            data
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            Self {
                data: Cursor::new(data[1..].to_vec()),
                id: data[0],
            }
        }
    }

    #[derive(Debug)]
    pub struct DeviceConfigurationDescriptor {
        command: Command,
        pub vid: u16,
        pub pid: u16,
        pub device_name: String,
        pub mode_count: u8,
        pub shift_mode_count: u8,
        pub button_name_vec: Vec<String>,
    }

    impl DeviceConfigurationDescriptor {
        pub fn new(
            vid: u16,
            pid: u16,
            device_name: String,
            mode_count: u8,
            shift_mode_count: u8,
            button_name_vec: Vec<String>,
        ) -> Self {
            let mut command = Command::new(0);

            command.add_u32(((vid as u32) << 16) + pid as u32);
            command.add_string(device_name.clone());
            command.add_byte(mode_count);
            command.add_byte(shift_mode_count);
            command.add_byte(button_name_vec.len() as u8);

            for button_name in button_name_vec.clone() {
                command.add_string(button_name);
            }

            Self {
                command,
                vid,
                pid,
                device_name,
                mode_count,
                shift_mode_count,
                button_name_vec,
            }
        }
    }

    impl CommandTrait for DeviceConfigurationDescriptor {
        fn to_bytes(&mut self) -> Vec<u8> {
            self.command.to_bytes()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            let mut self_ = Self {
                command: Command::from_bytes(data),
                vid: 0,
                pid: 0,
                device_name: String::new(),
                mode_count: 0,
                shift_mode_count: 0,
                button_name_vec: vec![],
            };
            let vid_pid = self_.command.get_u32();

            self_.vid = (vid_pid >> 16) as u16;
            self_.pid = vid_pid as u16;
            self_.device_name = self_.command.get_string();
            self_.mode_count = self_.command.get_byte();
            self_.shift_mode_count = self_.command.get_byte();

            let button_count = self_.command.get_byte();

            for _ in 0..button_count {
                self_.button_name_vec.push(self_.command.get_string());
            }

            self_
        }
    }
}
