mod window;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, ThreadId};
use std::time::Duration;

use util::connection::{command::*, Server};
use util::thread::{kill_double, DualChannel};
use util::time::Timer;
use webrender::webrender_api::*;

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
    /*
    if !kill_double() {
        let server = Server::new();
        let server_dualchannel = server.dual_channel;
        let driver_hashmap_mutex = Arc::new(Mutex::new(HashMap::<ThreadId, Driver>::new()));

        run_connection(server_dualchannel, driver_hashmap_mutex);
    }
    */

    let mut window_options = window::WindowOptions::new("test", 800, 600, Some("./ui/icon.png"));
    window_options.transparent = true;

    let mut window = window::Window::new(
        window_options,
        Some(ColorF::from(ColorU::new(33, 33, 33, 250))),
        0,
    );
    let mut timer = Timer::new(Duration::from_micros(3333));

    window.load_font_file(
        "OpenSans",
        "./ui/font/OpenSans.ttf",
        webrender::webrender_api::units::Au::from_f64_px(32.0),
    );

    loop {
        if window.tick() {
            break;
        }
        timer.wait();
    }

    window.deinit();
}

// connection processing
/*
fn run_connection(
    server_dualchannel: DualChannel<(ThreadId, bool, Vec<u8>)>,
    driver_hashmap_mutex: Arc<Mutex<HashMap<ThreadId, Driver>>>,
) {
    spawn(move || {
        let mut timer = Timer::new(Duration::from_millis(100));

        loop {
            if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                let mut driver_hashmap = match driver_hashmap_mutex.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };

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

                                    update_device_list_ui(driver_hashmap_mutex.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // clearing driver data
                    driver_hashmap.remove(&thread_id);

                    update_device_list_ui(driver_hashmap_mutex.clone());
                }
            }

            timer.wait();
        }
    });
}

// ui processing
fn update_device_list_ui(driver_hashmap_mutex: Arc<Mutex<HashMap<ThreadId, Driver>>>) {}
*/
