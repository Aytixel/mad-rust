slint::include_modules!();

use std::collections::{HashMap, HashSet};
use std::thread::ThreadId;

use slint::{Timer, TimerMode};
use util::connection::{
    command::CommandId, command::DeviceConfigurationDescriptor, CommandTrait, Server,
};
use util::thread::kill_double;

struct Driver {
    device_configuration_descriptor: DeviceConfigurationDescriptor,
}

impl Driver {
    fn new(device_configuration_descriptor_data: Vec<u8>) -> Self {
        Self {
            device_configuration_descriptor: DeviceConfigurationDescriptor::from_bytes(
                device_configuration_descriptor_data,
            ),
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

        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let _ui = ui_handle.unwrap();

                if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                    if is_running
                        && data.len() > 0
                        && CommandId::DeviceConfigurationDescriptor.test(&data)
                    {
                        thread_id_hashset.insert(thread_id);
                        driver_hashmap.insert(thread_id, Driver::new(data));
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
            },
        );

        ui.run();
    }
}
