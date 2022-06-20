use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;

use crate::window::GlobalStateTrait;
use crate::{Driver, GlobalState};

use util::connection::{command::Commands, Server};
use util::time::Timer;

pub struct Connection {
    server: Server,
    global_state: Arc<GlobalState>,
}

impl Connection {
    pub fn new(global_state: Arc<GlobalState>) -> Self {
        let server = Server::new();

        Self {
            server,
            global_state,
        }
    }

    pub fn run(&self) {
        let global_state = self.global_state.clone();
        let server_dualchannel = self.server.dual_channel.clone();

        spawn(move || {
            let mut timer = Timer::new(Duration::from_millis(100));

            loop {
                if let Some((thread_id, is_running, data)) = server_dualchannel.recv() {
                    let mut driver_hashmap = match global_state.driver_hashmap_mutex.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    if is_running {
                        if data.len() > 0 {
                            match Commands::from(data) {
                                Commands::DriverConfigurationDescriptor(
                                    driver_configuration_descriptor,
                                ) => {
                                    // initiate driver data
                                    driver_hashmap.insert(
                                        thread_id,
                                        Driver::new(driver_configuration_descriptor),
                                    );
                                }
                                Commands::DeviceList(device_list) => {
                                    if let Some(driver) = driver_hashmap.get_mut(&thread_id) {
                                        driver.device_list = device_list;
                                    }

                                    global_state.request_redraw();
                                }
                                _ => {}
                            }
                        }
                    } else {
                        // clearing driver data
                        driver_hashmap.remove(&thread_id);
                        global_state.request_redraw();
                    }
                }

                timer.wait();
            }
        });
    }
}
