use std::sync::Arc;
use std::time::Duration;

use crate::window::GlobalStateTrait;
use crate::{ConnectionEvent, Driver, GlobalState};

use tokio::{spawn, time};
use util::connection::command::{CommandTrait, Commands, RequestDeviceConfig};
use util::connection::{ConnectionState, Server};
use util::thread::MutexTrait;

pub struct Connection {
    server: Server,
    global_state: Arc<GlobalState>,
}

impl Connection {
    pub async fn new(global_state: Arc<GlobalState>) -> Self {
        let server = Server::new().await;

        Self {
            server,
            global_state,
        }
    }

    pub async fn run(&self) {
        {
            let global_state = self.global_state.clone();
            let server_dualchannel = self.server.dual_channel.clone();

            spawn(async move {
                let mut interval = time::interval(Duration::from_millis(100));

                loop {
                    // send data to clients
                    {
                        if let Some(connection_event) = global_state.pop_connection_event() {
                            match connection_event {
                                ConnectionEvent::RequestDeviceConfig(device_id) => {
                                    server_dualchannel
                                        .send_async((
                                            device_id.socket_addr,
                                            ConnectionState::Data(
                                                RequestDeviceConfig::new(device_id.serial_number)
                                                    .to_bytes(),
                                            ),
                                        ))
                                        .await
                                        .ok();
                                }
                                ConnectionEvent::ApplyDeviceConfig(
                                    socket_addr,
                                    mut device_config,
                                ) => {
                                    server_dualchannel
                                        .send_async((
                                            socket_addr,
                                            ConnectionState::Data(device_config.to_bytes()),
                                        ))
                                        .await
                                        .ok();
                                }
                            }
                        }
                    }

                    interval.tick().await;
                }
            });
        }

        {
            let global_state = self.global_state.clone();
            let server_dualchannel = self.server.dual_channel.clone();

            spawn(async move {
                loop {
                    // receive data from clients
                    if let Ok((socket_addr, connection_state)) =
                        server_dualchannel.recv_async().await
                    {
                        let mut driver_hashmap = global_state.driver_hashmap_mutex.lock_poisoned();

                        match connection_state {
                            ConnectionState::Start => {}
                            ConnectionState::Data(data) => {
                                if data.len() > 0 {
                                    match Commands::from(data) {
                                        Commands::DriverConfigurationDescriptor(
                                            driver_configuration_descriptor,
                                        ) => {
                                            // initiate driver data
                                            driver_hashmap.insert(
                                                socket_addr,
                                                Driver::new(driver_configuration_descriptor),
                                            );
                                        }
                                        Commands::DeviceList(device_list) => {
                                            if let Some(driver) =
                                                driver_hashmap.get_mut(&socket_addr)
                                            {
                                                driver.device_list = device_list;
                                            }

                                            global_state.request_redraw();
                                        }
                                        Commands::DeviceConfig(device_config) => {
                                            let mut selected_device_config_option = global_state
                                                .selected_device_config_option_mutex
                                                .lock_poisoned();

                                            *selected_device_config_option = Some(device_config);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            ConnectionState::End => {
                                // clearing driver data
                                driver_hashmap.remove(&socket_addr);
                                global_state.request_redraw();
                            }
                        }
                    }
                }
            });
        }
    }
}
