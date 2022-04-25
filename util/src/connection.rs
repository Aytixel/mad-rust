pub use server::Server;

mod server {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread::spawn;

    pub struct Server {
        tx: Sender<String>,
        rx: Receiver<String>,
    }

    impl Server {
        pub fn new() -> Self {
            let (server_tx, rx) = channel();
            let (tx, server_rx) = channel();

            spawn(move || {});

            Self { tx: tx, rx: rx }
        }
    }
}

pub use client::Client;

mod client {
    use sysinfo::{ProcessExt, System, SystemExt};

    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread::spawn;

    pub struct Client {
        tx: Sender<String>,
        rx: Receiver<String>,
    }

    impl Client {
        pub fn new() -> Self {
            let (server_tx, rx) = channel();
            let (tx, server_rx) = channel();
            let mut sys = System::new_all();

            spawn(move || {
                sys.refresh_all();

                // Number of processors:
                println!("NB processors: {}", sys.processors().len());

                // Display processes ID, name na disk usage:
                for (pid, process) in sys.processes() {
                    println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
                }
            });

            Self { tx: tx, rx: rx }
        }
    }
}
