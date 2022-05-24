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
                                    let mut timer = Timer::new(Duration::from_millis(100));
                                    let mut size_buffer = [0; 8];
                                    let mut last_packet_send = Instant::now();
                                    let mut last_packet_receive = Instant::now();
                                    let thread_id = current().id();

                                    child.send((thread_id, true, vec![]));

                                    'main: loop {
                                        // timeout packet
                                        if last_packet_receive.elapsed() > Duration::from_secs(5) {
                                            child.send((thread_id, false, vec![]));
                                            break;
                                        }

                                        // life packet
                                        if last_packet_send.elapsed() > Duration::from_secs(1) {
                                            socket.write_all(&u64::MAX.to_be_bytes()).ok();

                                            last_packet_send = Instant::now();
                                        }

                                        // data from the client
                                        if let Ok(_) = socket.read_exact(&mut size_buffer) {
                                            let size = u64::from_be_bytes(size_buffer);

                                            // connection end
                                            if size == 0 {
                                                child.send((thread_id, false, vec![]));
                                                break;
                                            }

                                            // life packet
                                            last_packet_receive = Instant::now();

                                            if size != u64::MAX {
                                                let mut buffer = vec![0; size as usize];

                                                if let Ok(_) = socket.read_exact(&mut buffer) {
                                                    child.send((thread_id, true, buffer));
                                                }
                                            }
                                        }

                                        // data to the client
                                        {
                                            let mut buffer = child.lock_rx();
                                            let mut i = 0;

                                            while i < buffer.len() {
                                                let (thread_id_, is_running, data) =
                                                    buffer[i].clone();

                                                if thread_id_ == thread_id {
                                                    // connection end
                                                    if !is_running {
                                                        socket.write_all(&0u64.to_be_bytes()).ok();
                                                        break 'main;
                                                    }

                                                    socket
                                                        .write_all(
                                                            &(data.len() as u64).to_be_bytes(),
                                                        )
                                                        .ok();
                                                    socket.write_all(&data).ok();
                                                    buffer.remove(i);
                                                } else {
                                                    i += 1;
                                                }
                                            }
                                        }

                                        timer.wait();
                                    }

                                    // clearing old data
                                    {
                                        let mut buffer = child.lock_rx();
                                        let mut i = 0;

                                        while i < buffer.len() {
                                            let (thread_id_, _, _) = buffer[i].clone();

                                            if thread_id_ == thread_id {
                                                buffer.remove(i);
                                            } else {
                                                i += 1;
                                            }
                                        }
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

                            // clearing old data
                            {
                                let mut buffer = child.lock_rx();

                                buffer.clear();
                            }

                            child.send((true, vec![]));

                            'main: loop {
                                // timeout packet
                                if last_packet_receive.elapsed() > Duration::from_secs(5) {
                                    child.send((false, vec![]));
                                    break;
                                }

                                // life packet
                                if last_packet_send.elapsed() > Duration::from_secs(1) {
                                    socket.write_all(&u64::MAX.to_be_bytes()).ok();

                                    last_packet_send = Instant::now();
                                }

                                // data from the server
                                if let Ok(_) = socket.read_exact(&mut size_buffer) {
                                    let size = u64::from_be_bytes(size_buffer);

                                    // connection end
                                    if size == 0 {
                                        child.send((false, vec![]));
                                        break;
                                    }

                                    // life packet
                                    last_packet_receive = Instant::now();

                                    if size != u64::MAX {
                                        let mut buffer = vec![0; size as usize];

                                        if let Ok(_) = socket.read_exact(&mut buffer) {
                                            child.send((true, buffer));
                                        }
                                    }
                                }

                                // data to the server
                                loop {
                                    if let Some((is_running, data)) = child.recv() {
                                        // connection end
                                        if !is_running {
                                            socket.write_all(&0u64.to_be_bytes()).ok();
                                            break 'main;
                                        }

                                        socket.write_all(&(data.len() as u64).to_be_bytes()).ok();
                                        socket.write_all(&data).ok();

                                        last_packet_send = Instant::now();
                                    } else {
                                        break;
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

    #[derive(Debug, Clone, Default)]
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

    const DEVICE_CONFIGURATION_DESCRIPTOR_ID: u8 = 0;
    const DEVICE_LIST_ID: u8 = 1;
    const UNKNOWN_ID: u8 = 255;

    #[derive(Debug, Clone)]
    pub enum Commands {
        DeviceConfigurationDescriptor(DeviceConfigurationDescriptor),
        DeviceList(DeviceList),
        Unknown,
    }

    impl Commands {
        pub fn test(self, value: &Vec<u8>) -> bool {
            value[0] == self.into()
        }
    }

    impl Into<u8> for Commands {
        fn into(self) -> u8 {
            match self {
                Self::DeviceConfigurationDescriptor(_) => DEVICE_CONFIGURATION_DESCRIPTOR_ID,
                Self::DeviceList(_) => DEVICE_LIST_ID,
                Self::Unknown => UNKNOWN_ID,
            }
        }
    }

    impl From<u8> for Commands {
        fn from(value: u8) -> Self {
            match value {
                DEVICE_CONFIGURATION_DESCRIPTOR_ID => {
                    Self::DeviceConfigurationDescriptor(DeviceConfigurationDescriptor::default())
                }
                DEVICE_LIST_ID => Self::DeviceList(DeviceList::default()),
                _ => Self::Unknown,
            }
        }
    }

    impl From<Vec<u8>> for Commands {
        fn from(value: Vec<u8>) -> Self {
            match value[0] {
                DEVICE_CONFIGURATION_DESCRIPTOR_ID => Self::DeviceConfigurationDescriptor(
                    DeviceConfigurationDescriptor::from_bytes(value),
                ),
                DEVICE_LIST_ID => Self::DeviceList(DeviceList::from_bytes(value)),
                _ => Self::Unknown,
            }
        }
    }

    impl PartialEq<u8> for Commands {
        fn eq(&self, other: &u8) -> bool {
            let value: u8 = self.clone().into();

            value == *other
        }
    }

    impl PartialEq<Vec<u8>> for Commands {
        fn eq(&self, other: &Vec<u8>) -> bool {
            *self == other[0]
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct DeviceConfigurationDescriptor {
        command: Command,
        pub vid: u16,
        pub pid: u16,
        pub device_name: String,
        pub device_icon_path: String,
        pub mode_count: u8,
        pub shift_mode_count: u8,
        pub button_name_vec: Vec<String>,
    }

    impl DeviceConfigurationDescriptor {
        pub fn new(
            vid: u16,
            pid: u16,
            device_name: String,
            device_icon_path: String,
            mode_count: u8,
            shift_mode_count: u8,
            button_name_vec: Vec<String>,
        ) -> Self {
            let mut command = Command::new(DEVICE_CONFIGURATION_DESCRIPTOR_ID);

            command.add_u32(((vid as u32) << 16) + pid as u32);
            command.add_string(device_name.clone());
            command.add_string(device_icon_path.clone());
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
                device_icon_path,
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
                device_icon_path: String::new(),
                mode_count: 0,
                shift_mode_count: 0,
                button_name_vec: vec![],
            };
            let vid_pid = self_.command.get_u32();

            self_.vid = (vid_pid >> 16) as u16;
            self_.pid = vid_pid as u16;
            self_.device_name = self_.command.get_string();
            self_.device_icon_path = self_.command.get_string();
            self_.mode_count = self_.command.get_byte();
            self_.shift_mode_count = self_.command.get_byte();

            let button_count = self_.command.get_byte();

            for _ in 0..button_count {
                self_.button_name_vec.push(self_.command.get_string());
            }

            self_
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct DeviceList {
        command: Command,
        pub serial_number_vec: Vec<String>,
    }

    impl DeviceList {
        pub fn new(serial_number_vec: Vec<String>) -> Self {
            let mut command = Command::new(DEVICE_LIST_ID);

            command.add_byte(serial_number_vec.len() as u8);

            for serial_number in serial_number_vec.clone() {
                command.add_string(serial_number);
            }

            Self {
                command,
                serial_number_vec,
            }
        }
    }

    impl CommandTrait for DeviceList {
        fn to_bytes(&mut self) -> Vec<u8> {
            self.command.to_bytes()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            let mut self_ = Self {
                command: Command::from_bytes(data),
                serial_number_vec: vec![],
            };
            let serial_number_count = self_.command.get_byte();

            for _ in 0..serial_number_count {
                self_.serial_number_vec.push(self_.command.get_string());
            }

            self_
        }
    }
}
