slint::include_modules!();

use std::collections::HashSet;

use slint::{Timer, TimerMode};
use util::connection::{command::DeviceConfigurationDescriptor, CommandTrait, Server};
use util::thread::kill_double;

fn main() {
    if !kill_double() {
        let server = Server::new();
        let server_dualchannel = server.dual_channel;
        let ui = MainWindow::new();
        let ui_handle = ui.as_weak();
        let timer = Timer::default();
        let mut thread_id_vec = HashSet::new();
        let mut test: u16 = 0;

        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let _ui = ui_handle.unwrap();

                if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                    if is_running && data.len() > 0 {
                        thread_id_vec.insert(thread_id);
                        println!("{:?}", DeviceConfigurationDescriptor::from_bytes(data));
                    }

                    if !is_running {
                        thread_id_vec.remove(&thread_id);
                    }
                }

                for thread_id in thread_id_vec.clone() {
                    server_dualchannel.send((thread_id, true, vec![test as u8, 42, 125]));
                }

                test += 1;
                if test == 256 {
                    test = 0;
                }
            },
        );

        ui.run();
    }
}
