slint::include_modules!();

use std::collections::{HashMap, HashSet};
use std::thread::ThreadId;
use std::time::Duration;

use slint::{Timer, TimerMode};
use util::connection::{command::*, Server};
use util::thread::kill_double;

#[derive(Debug)]
struct Driver {
    device_configuration_descriptor: DeviceConfigurationDescriptor,
    device_list: DeviceList,
}

impl Driver {
    fn new(device_configuration_descriptor: DeviceConfigurationDescriptor) -> Self {
        Self {
            device_configuration_descriptor: device_configuration_descriptor,
            device_list: DeviceList::default(),
        }
    }
}

fn main() {
    if !kill_double() {
        let server = Server::new();
        let mut server_dualchannel = server.dual_channel;
        let ui = MainWindow::new();
        let ui_handle = ui.as_weak();
        let timer = Timer::default();
        let mut thread_id_hashset = HashSet::new();
        let mut driver_hashmap = HashMap::<ThreadId, Driver>::new();

        timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
            let _ui = ui_handle.unwrap();

            if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                if is_running && data.len() > 0 {
                    match Commands::from(data) {
                        Commands::DeviceConfigurationDescriptor(
                            device_configuration_descriptor,
                        ) => {
                            thread_id_hashset.insert(thread_id);
                            driver_hashmap
                                .insert(thread_id, Driver::new(device_configuration_descriptor));
                        }
                        Commands::DeviceList(device_list) => {
                            driver_hashmap.get_mut(&thread_id).unwrap().device_list = device_list;
                        }
                        _ => {}
                    }
                }

                if !is_running {
                    // clearing old thread data
                    thread_id_hashset.remove(&thread_id);
                    driver_hashmap.remove(&thread_id);

                    if let Some(mut buffer) = server_dualchannel.lock_tx() {
                        let mut i = 0;

                        while i < buffer.len() {
                            let (thread_id_deleted, _, _) = buffer[i].clone();

                            if thread_id == thread_id_deleted {
                                buffer.remove(i);
                            } else {
                                i += 1;
                            }
                        }
                    }

                    server_dualchannel.unlock_tx();
                }
            }
        });

        ui.run();
    }
}
