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

pub use command::CommandTrait;

pub mod command {
    use serde::{Deserialize, Serialize};

    pub trait CommandTrait {
        fn to_bytes(&mut self) -> Vec<u8>;

        fn from_bytes(data: Vec<u8>) -> Self;
    }

    const DRIVER_CONFIGURATION_DESCRIPTOR_ID: u8 = 0;
    const DEVICE_LIST_ID: u8 = 1;
    const UNKNOWN_ID: u8 = 255;

    #[derive(Debug, Clone)]
    pub enum Commands {
        DriverConfigurationDescriptor(DriverConfigurationDescriptor),
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
                Self::DriverConfigurationDescriptor(_) => DRIVER_CONFIGURATION_DESCRIPTOR_ID,
                Self::DeviceList(_) => DEVICE_LIST_ID,
                Self::Unknown => UNKNOWN_ID,
            }
        }
    }

    impl From<u8> for Commands {
        fn from(value: u8) -> Self {
            match value {
                DRIVER_CONFIGURATION_DESCRIPTOR_ID => {
                    Self::DriverConfigurationDescriptor(DriverConfigurationDescriptor::default())
                }
                DEVICE_LIST_ID => Self::DeviceList(DeviceList::default()),
                _ => Self::Unknown,
            }
        }
    }

    impl From<Vec<u8>> for Commands {
        fn from(value: Vec<u8>) -> Self {
            match value[0] {
                DRIVER_CONFIGURATION_DESCRIPTOR_ID => Self::DriverConfigurationDescriptor(
                    DriverConfigurationDescriptor::from_bytes(value),
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

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct DriverConfigurationDescriptor {
        #[serde(skip_serializing)]
        pub id: u8,
        pub vid: u16,
        pub pid: u16,
        pub device_name: String,
        pub device_icon: Vec<u8>,
        pub mode_count: u8,
        pub shift_mode_count: u8,
        pub button_name_vec: Vec<String>,
    }

    impl DriverConfigurationDescriptor {
        pub fn new(
            vid: u16,
            pid: u16,
            device_name: String,
            device_icon: Vec<u8>,
            mode_count: u8,
            shift_mode_count: u8,
            button_name_vec: Vec<String>,
        ) -> Self {
            Self {
                id: DRIVER_CONFIGURATION_DESCRIPTOR_ID,
                vid,
                pid,
                device_name,
                device_icon,
                mode_count,
                shift_mode_count,
                button_name_vec,
            }
        }
    }

    impl CommandTrait for DriverConfigurationDescriptor {
        fn to_bytes(&mut self) -> Vec<u8> {
            let mut data = vec![self.id];

            data.append(&mut bincode::serialize(&self).unwrap());
            data
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data[1..]).unwrap()
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct DeviceList {
        #[serde(skip_serializing)]
        pub id: u8,
        pub serial_number_vec: Vec<String>,
    }

    impl DeviceList {
        pub fn new(serial_number_vec: Vec<String>) -> Self {
            Self {
                id: DEVICE_LIST_ID,
                serial_number_vec,
            }
        }
    }

    impl CommandTrait for DeviceList {
        fn to_bytes(&mut self) -> Vec<u8> {
            let mut data = vec![self.id];

            data.append(&mut bincode::serialize(&self).unwrap());
            data
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data[1..]).unwrap()
        }
    }
}
