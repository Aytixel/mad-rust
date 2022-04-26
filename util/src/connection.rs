pub use server::Server;

mod server {
    use std::thread::spawn;

    use crate::thread::DualChannel;

    pub struct Server {
        pub dual_channel: DualChannel<String>,
    }

    impl Server {
        pub fn new() -> Self {
            let (host, child) = DualChannel::new();

            spawn(move || {});

            Self { dual_channel: host }
        }
    }
}

pub use client::Client;

mod client {
    use std::thread::spawn;

    use crate::thread::DualChannel;

    pub struct Client {
        pub dual_channel: DualChannel<String>,
    }

    impl Client {
        pub fn new() -> Self {
            let (host, child) = DualChannel::new();

            spawn(move || {});

            Self { dual_channel: host }
        }
    }
}
