slint::include_modules!();

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;
use std::thread::ThreadId;
use std::time::Duration;

use slint::{Image, SharedString, Timer, TimerMode, VecModel};
use util::connection::{command::*, Server};
use util::thread::kill_double;

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

#[derive(Debug, Clone)]
enum AppState {
    DeviceSelectionWindow,
}

impl Into<i32> for AppState {
    fn into(self) -> i32 {
        self as i32
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

        ui.set_app_state(AppState::DeviceSelectionWindow.into());

        timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
            let ui = ui_handle.upgrade().unwrap();

            if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                if is_running {
                    if data.len() > 0 {
                        match Commands::from(data) {
                            Commands::DeviceConfigurationDescriptor(
                                device_configuration_descriptor,
                            ) => {
                                // initiate driver data
                                thread_id_hashset.insert(thread_id);
                                driver_hashmap.insert(
                                    thread_id,
                                    Driver::new(device_configuration_descriptor),
                                );
                            }
                            Commands::DeviceList(device_list) => {
                                if let Some(driver) = driver_hashmap.get_mut(&thread_id) {
                                    driver.device_list = device_list;

                                    let mut device_list: Vec<DeviceData> = vec![];

                                    for driver in driver_hashmap.values() {
                                        if let Ok(icon) = Image::load_from_path(Path::new(
                                            &driver
                                                .device_configuration_descriptor
                                                .device_icon_path,
                                        )) {
                                            for serial_number in
                                                driver.device_list.serial_number_vec.clone()
                                            {
                                                device_list.push(DeviceData {
                                                    icon: icon.clone(),
                                                    name: SharedString::from(
                                                        driver
                                                            .device_configuration_descriptor
                                                            .device_name
                                                            .clone(),
                                                    ),
                                                    serial_number: SharedString::from(
                                                        serial_number,
                                                    ),
                                                });
                                            }
                                        }
                                    }

                                    ui.set_device_list(
                                        Rc::new(VecModel::from(device_list)).clone().into(),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
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
