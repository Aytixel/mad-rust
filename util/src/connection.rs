pub use server::Server;

pub mod server {
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use crate::thread::DualChannel;
    use crate::time::{Timer, TIMEOUT_1S};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::spawn;
    use tokio::sync::Mutex;

    pub struct Server {
        pub dual_channel: DualChannel<(SocketAddr, bool, Vec<u8>)>,
    }

    impl Server {
        pub async fn new() -> Self {
            let (host, child) = DualChannel::<(SocketAddr, bool, Vec<u8>)>::new();

            spawn(async move {
                if let Ok(listener) = TcpListener::bind("127.0.0.1:651").await {
                    loop {
                        if let Ok((socket, socket_addr)) = listener.accept().await {
                            let child = child.clone();
                            let socket_mutex = Arc::new(Mutex::new(socket));

                            spawn(async move {
                                // data communication handling
                                let last_packet_receive_mutex =
                                    Arc::new(Mutex::new(Instant::now()));
                                let running = Arc::new(AtomicBool::new(true));

                                child.send_async((socket_addr, true, vec![])).await.ok();

                                {
                                    let child = child.clone();
                                    let socket_mutex = socket_mutex.clone();
                                    let last_packet_receive_mutex =
                                        last_packet_receive_mutex.clone();
                                    let running = running.clone();

                                    spawn(async move {
                                        let mut timer = Timer::new(TIMEOUT_1S);

                                        while running.load(Ordering::SeqCst) {
                                            // timeout packet
                                            if last_packet_receive_mutex.lock().await.elapsed()
                                                > Duration::from_secs(5)
                                            {
                                                running.store(false, Ordering::SeqCst);
                                                child
                                                    .send_async((socket_addr, false, vec![]))
                                                    .await
                                                    .ok();
                                                break;
                                            }

                                            // life packet
                                            socket_mutex
                                                .lock()
                                                .await
                                                .write_all(&u64::MAX.to_be_bytes())
                                                .await
                                                .ok();
                                            timer.wait_async().await;
                                        }
                                    });
                                }

                                {
                                    let child = child.clone();
                                    let socket_mutex = socket_mutex.clone();
                                    let running = running.clone();

                                    spawn(async move {
                                        let mut size_buffer = [0; 8];

                                        // data from the client
                                        while running.load(Ordering::SeqCst) {
                                            let mut socket = socket_mutex.lock().await;

                                            if let Ok(_) = socket.read_exact(&mut size_buffer).await
                                            {
                                                let size = u64::from_be_bytes(size_buffer);

                                                // connection end
                                                if size == 0 {
                                                    running.store(false, Ordering::SeqCst);
                                                    child
                                                        .send_async((socket_addr, false, vec![]))
                                                        .await
                                                        .ok();
                                                    break;
                                                }

                                                // life packet
                                                *last_packet_receive_mutex.lock().await =
                                                    Instant::now();

                                                // if the packet is bigger than 20 Megabyte it's considered as life packet
                                                if size < 20000000 {
                                                    let mut buffer = vec![0; size as usize];

                                                    if let Ok(_) =
                                                        socket.read_exact(&mut buffer).await
                                                    {
                                                        child
                                                            .send_async((socket_addr, true, buffer))
                                                            .await
                                                            .ok();
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }

                                // data to the client
                                while running.load(Ordering::SeqCst) {
                                    if let Ok((socket_addr_, is_running, data)) =
                                        child.recv_async().await
                                    {
                                        if socket_addr_ == socket_addr {
                                            let mut socket = socket_mutex.lock().await;

                                            // connection end
                                            if !is_running {
                                                running.store(false, Ordering::SeqCst);
                                                socket.write_all(&0u64.to_be_bytes()).await.ok();
                                                break;
                                            }

                                            socket
                                                .write_all(&(data.len() as u64).to_be_bytes())
                                                .await
                                                .ok();
                                            socket.write_all(&data).await.ok();
                                        }
                                    }
                                }
                            });
                        }
                    }
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
            let (host, child) = DualChannel::<(bool, Vec<u8>)>::new();

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
                                // timeout packet
                                if last_packet_receive.elapsed() > Duration::from_secs(5) {
                                    child.send((false, vec![])).ok();
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
                                        child.send((false, vec![])).ok();
                                        break;
                                    }

                                    // life packet
                                    last_packet_receive = Instant::now();

                                    // if the packet is bigger than 20 Megabyte it's considered as life packet
                                    if size < 20000000 {
                                        let mut buffer = vec![0; size as usize];

                                        if let Ok(_) = socket.read_exact(&mut buffer) {
                                            child.send((true, buffer)).ok();
                                        }
                                    }
                                }

                                // data to the server
                                while let Ok(Some((is_running, data))) = child.try_recv() {
                                    // connection end
                                    if !is_running {
                                        socket.write_all(&0u64.to_be_bytes()).ok();
                                        break 'main;
                                    }

                                    socket.write_all(&(data.len() as u64).to_be_bytes()).ok();
                                    socket.write_all(&data).ok();

                                    last_packet_send = Instant::now();
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
    const REQUEST_DEVICE_CONFIG_ID: u8 = 2;
    const DEVICE_CONFIG_ID: u8 = 3;
    const UNKNOWN_ID: u8 = 255;

    #[derive(Debug, Clone)]
    pub enum Commands {
        DriverConfigurationDescriptor(DriverConfigurationDescriptor),
        DeviceList(DeviceList),
        RequestDeviceConfig(RequestDeviceConfig),
        DeviceConfig(DeviceConfig),
        Unknown,
    }

    impl Commands {
        pub fn test(self, value: &Vec<u8>) -> bool {
            self.into() == value[0]
        }
    }

    impl Commands {
        fn into(self) -> u8 {
            match self {
                Self::DriverConfigurationDescriptor(_) => DRIVER_CONFIGURATION_DESCRIPTOR_ID,
                Self::DeviceList(_) => DEVICE_LIST_ID,
                Self::RequestDeviceConfig(_) => REQUEST_DEVICE_CONFIG_ID,
                Self::DeviceConfig(_) => DEVICE_CONFIG_ID,
                Self::Unknown => UNKNOWN_ID,
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
                REQUEST_DEVICE_CONFIG_ID => {
                    Self::RequestDeviceConfig(RequestDeviceConfig::from_bytes(value))
                }
                DEVICE_CONFIG_ID => Self::DeviceConfig(DeviceConfig::from_bytes(value)),
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
            bincode::serialize(&self).unwrap()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data).unwrap()
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct DeviceList {
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
            bincode::serialize(&self).unwrap()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data).unwrap()
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct RequestDeviceConfig {
        pub id: u8,
        pub serial_number: String,
    }

    impl RequestDeviceConfig {
        pub fn new(serial_number: String) -> Self {
            Self {
                id: REQUEST_DEVICE_CONFIG_ID,
                serial_number,
            }
        }
    }

    impl CommandTrait for RequestDeviceConfig {
        fn to_bytes(&mut self) -> Vec<u8> {
            bincode::serialize(&self).unwrap()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data).unwrap()
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct DeviceConfig {
        pub id: u8,
        pub serial_number: String,
        pub config: Vec<[Vec<String>; 2]>,
    }

    impl DeviceConfig {
        pub fn new(serial_number: String, config: Vec<[Vec<String>; 2]>) -> Self {
            Self {
                id: DEVICE_CONFIG_ID,
                serial_number,
                config,
            }
        }
    }

    impl CommandTrait for DeviceConfig {
        fn to_bytes(&mut self) -> Vec<u8> {
            bincode::serialize(&self).unwrap()
        }

        fn from_bytes(data: Vec<u8>) -> Self {
            bincode::deserialize(&data).unwrap()
        }
    }
}
