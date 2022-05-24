slint::include_modules!();

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, ThreadId};
use std::time::Duration;

use slint::{invoke_from_event_loop, Image, SharedString, VecModel, Weak};
use util::connection::{command::*, Server};
use util::thread::kill_double;
use util::time::Timer;

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
        let driver_hashmap_mutex = Arc::new(Mutex::new(HashMap::<ThreadId, Driver>::new()));

        ui.set_app_state(AppState::DeviceSelectionWindow.into());

        spawn(move || {
            let mut timer = Timer::new(Duration::from_millis(100));

            loop {
                if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                    let mut driver_hashmap = driver_hashmap_mutex.lock().unwrap();

                    if is_running {
                        if data.len() > 0 {
                            match Commands::from(data) {
                                Commands::DeviceConfigurationDescriptor(
                                    device_configuration_descriptor,
                                ) => {
                                    // initiate driver data
                                    driver_hashmap.insert(
                                        thread_id,
                                        Driver::new(device_configuration_descriptor),
                                    );
                                }
                                Commands::DeviceList(device_list) => {
                                    if let Some(driver) = driver_hashmap.get_mut(&thread_id) {
                                        driver.device_list = device_list;

                                        update_device_list_ui(
                                            ui_handle.clone(),
                                            driver_hashmap_mutex.clone(),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        // clearing old thread data
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

                        update_device_list_ui(ui_handle.clone(), driver_hashmap_mutex.clone());
                    }
                }

                timer.wait();
            }
        });

        ui.run();
    }
}

fn update_device_list_ui(
    ui_handle: Weak<MainWindow>,
    driver_hashmap_mutex: Arc<Mutex<HashMap<ThreadId, Driver>>>,
) {
    invoke_from_event_loop(move || {
        let mut device_list: Vec<DeviceData> = vec![];

        for driver in driver_hashmap_mutex.lock().unwrap().values() {
            if let Ok(icon) = Image::load_from_path(Path::new(
                &driver.device_configuration_descriptor.device_icon_path,
            )) {
                for serial_number in driver.device_list.serial_number_vec.clone() {
                    device_list.push(DeviceData {
                        icon: icon.clone(),
                        name: SharedString::from(
                            driver.device_configuration_descriptor.device_name.clone(),
                        ),
                        serial_number: SharedString::from(serial_number),
                    });
                }
            }
        }

        ui_handle
            .upgrade()
            .unwrap()
            .set_device_list(Rc::new(VecModel::from(device_list)).clone().into());
    });
}
