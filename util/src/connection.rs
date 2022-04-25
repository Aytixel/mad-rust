pub use server::Server;

mod server {
    use std::thread::spawn;

    use crate::thread::DualChannel;

    pub struct Server {
        dual_channel: DualChannel<String>,
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
    use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

    use std::thread::spawn;

    use crate::thread::DualChannel;

    pub struct Client {
        dual_channel: DualChannel<String>,
    }

    impl Client {
        pub fn new() -> Self {
            let (host, child) = DualChannel::new();
            let mut sys = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );

            spawn(move || {
                sys.refresh_processes_specifics(ProcessRefreshKind::new());

                for process in sys.processes_by_name("mad-rust") {
                    println!("[{}] {:?}", process.pid(), process.exe());
                }
            });

            Self { dual_channel: host }
        }
    }
}
