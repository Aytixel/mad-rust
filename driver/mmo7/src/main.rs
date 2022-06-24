// hide the console on release builds for windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mapper;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;

use hashbrown::HashSet;
use mapper::Mapper;
use rusb::{Context, DeviceHandle, UsbContext};
use serde::{Deserialize, Serialize};
use thread_priority::{set_current_thread_priority, ThreadPriority};
use util::config::ConfigManager;
use util::connection::{command::*, Client};
use util::thread::{kill_double, DualChannel, MutexTrait};
use util::time::{Timer, TIMEOUT_1S};

const VID: u16 = 0x0738;
const PID: u16 = 0x1713;

#[derive(Deserialize, Serialize, Default)]
pub struct ButtonConfigs {
    scroll_button: [[String; 3]; 2],
    left_actionlock: [[String; 3]; 2],
    right_actionlock: [[String; 3]; 2],
    forwards_button: [[String; 3]; 2],
    back_button: [[String; 3]; 2],
    thumb_anticlockwise: [[String; 3]; 2],
    thumb_clockwise: [[String; 3]; 2],
    hat_top: [[String; 3]; 2],
    hat_left: [[String; 3]; 2],
    hat_right: [[String; 3]; 2],
    hat_bottom: [[String; 3]; 2],
    button_1: [[String; 3]; 2],
    precision_aim: [[String; 3]; 2],
    button_2: [[String; 3]; 2],
    button_3: [[String; 3]; 2],
}

type MousesConfig = BTreeMap<String, ButtonConfigs>;

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

#[derive(Debug, Clone)]
enum Message {
    DeviceListUpdate,
}

fn main() {
    if !kill_double() {
        let client = Client::new();
        let client_dualchannel = client.dual_channel;
        let device_list_mutex = Arc::new(Mutex::new(HashSet::<String>::new()));
        let (host, child) = DualChannel::<Message>::new();
        let icon_data = include_bytes!("../icon.png").to_vec();
        let mouse_configs_mutex = Arc::new(Mutex::new(ConfigManager::<MousesConfig>::new(
            "mmo7_profiles",
        )));

        watch_config_update(mouse_configs_mutex.clone());
        run_connection(
            client_dualchannel,
            child,
            device_list_mutex.clone(),
            icon_data,
            mouse_configs_mutex.clone(),
        );
        listening_new_device(host, device_list_mutex, mouse_configs_mutex);
    }
}

fn watch_config_update(mouse_configs_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>) {
    let mouse_configs_mutex = mouse_configs_mutex.clone();

    spawn(move || {
        set_current_thread_priority(ThreadPriority::Min).ok();

        let mut timer = Timer::new(TIMEOUT_1S * 10);

        loop {
            {
                let mut mouse_configs = mouse_configs_mutex.lock_safe();

                mouse_configs.update();
            }

            timer.wait();
        }
    });
}

// device handling
fn listening_new_device(
    host: DualChannel<Message>,
    device_list_mutex: Arc<Mutex<HashSet<String>>>,
    mouse_configs_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>,
) {
    let mut timer = Timer::new(TIMEOUT_1S);

    loop {
        if let Ok(context) = Context::new() {
            if let Ok(devices) = context.devices() {
                for device in devices.iter() {
                    if let Ok(device_descriptor) = device.device_descriptor() {
                        if device_descriptor.vendor_id() == VID
                            && device_descriptor.product_id() == PID
                        {
                            if let Ok(device_handle) = device.open() {
                                if let Ok(languages) = device_handle.read_languages(TIMEOUT_1S) {
                                    if let Ok(serial_number) = device_handle
                                        .read_serial_number_string(
                                            languages[0],
                                            &device_descriptor,
                                            TIMEOUT_1S,
                                        )
                                    {
                                        let mut device_list = device_list_mutex.lock_safe();

                                        if let None = device_list.get(&serial_number) {
                                            {
                                                // create a default config if needed
                                                let mut mouse_configs =
                                                    mouse_configs_mutex.lock_safe();

                                                if !mouse_configs
                                                    .config
                                                    .contains_key(&serial_number)
                                                {
                                                    mouse_configs.config.insert(
                                                        serial_number.clone(),
                                                        ButtonConfigs::default(),
                                                    );
                                                    mouse_configs.save();
                                                }
                                            }

                                            device_list.insert(serial_number.clone());

                                            let host = host.clone();
                                            let device_list_mutex = device_list_mutex.clone();
                                            let mouse_configs_mutex = mouse_configs_mutex.clone();

                                            spawn(move || {
                                                set_current_thread_priority(ThreadPriority::Max)
                                                    .ok();

                                                run_device(
                                                    serial_number.clone(),
                                                    host.clone(),
                                                    mouse_configs_mutex,
                                                );

                                                device_list_mutex
                                                    .lock_safe()
                                                    .remove(&serial_number);
                                                host.send(Message::DeviceListUpdate);
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        timer.wait();
    }
}

fn find_device(serial_number: String) -> Option<DeviceHandle<Context>> {
    if let Ok(context) = Context::new() {
        if let Ok(devices) = context.devices() {
            for device in devices.iter() {
                if let Ok(device_descriptor) = device.device_descriptor() {
                    if device_descriptor.vendor_id() == VID && device_descriptor.product_id() == PID
                    {
                        if let Ok(device_handle) = device.open() {
                            if let Ok(languages) = device_handle.read_languages(TIMEOUT_1S) {
                                if let Ok(serial_number_found) = device_handle
                                    .read_serial_number_string(
                                        languages[0],
                                        &device_descriptor,
                                        TIMEOUT_1S,
                                    )
                                {
                                    if serial_number == serial_number_found {
                                        return Some(device_handle);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn run_device(
    serial_number: String,
    dual_channel: DualChannel<Message>,
    mouse_configs_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>,
) {
    if let Some(mut device_handle) = find_device(serial_number.clone()) {
        let device = device_handle.device();
        if let Ok(config_descriptor) = device.config_descriptor(0) {
            if let Some(interface) = config_descriptor.interfaces().next() {
                if let Some(interface_descriptor) = interface.descriptors().next() {
                    if let Some(endpoint_descriptor) =
                        interface_descriptor.endpoint_descriptors().next()
                    {
                        let endpoint = Endpoint {
                            config: config_descriptor.number(),
                            iface: interface_descriptor.interface_number(),
                            setting: interface_descriptor.setting_number(),
                            address: endpoint_descriptor.address(),
                        };

                        let has_kernel_driver =
                            match device_handle.kernel_driver_active(endpoint.iface) {
                                Ok(true) => {
                                    device_handle.detach_kernel_driver(endpoint.iface).ok();
                                    true
                                }
                                _ => false,
                            };

                        if let (Ok(_), Ok(_), Ok(_)) = (
                            device_handle.set_active_configuration(endpoint.config),
                            device_handle.claim_interface(endpoint.iface),
                            device_handle.set_alternate_setting(endpoint.iface, endpoint.setting),
                        ) {
                            println!("{} connected", serial_number);

                            dual_channel.send(Message::DeviceListUpdate);

                            let mut buffer = [0; 8];
                            let mut mapper = Mapper::new(mouse_configs_mutex);

                            loop {
                                match device_handle.read_interrupt(
                                    endpoint.address,
                                    &mut buffer,
                                    Duration::from_millis(100),
                                ) {
                                    Ok(_) => {
                                        //println!("{} : {:?}", serial_number, buffer);
                                        mapper.emulate(&buffer);
                                    }
                                    Err(rusb::Error::Timeout) => {}
                                    Err(err) => {
                                        println!("{} disconnected : {}", serial_number, err);
                                        break;
                                    }
                                }
                            }

                            if has_kernel_driver {
                                device_handle.attach_kernel_driver(endpoint.iface).ok();
                            }
                        }
                    }
                }
            }
        }
    }
}

// connection processing
fn run_connection(
    client_dualchannel: DualChannel<(bool, Vec<u8>)>,
    child: DualChannel<Message>,
    device_list_mutex: Arc<Mutex<HashSet<String>>>,
    icon_data: Vec<u8>,
    mouse_configs_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>,
) {
    spawn(move || {
        set_current_thread_priority(ThreadPriority::Min).ok();

        let mut driver_configuration_descriptor = DriverConfigurationDescriptor::new(
            VID,
            PID,
            "MMO7".to_string(),
            icon_data,
            3,
            3,
            vec![
                "Scroll Button".to_string(),
                "Left ActionLock".to_string(),
                "Right ActionLock".to_string(),
                "Forwards Button".to_string(),
                "Back Button".to_string(),
                "Thumb Anticlockwise".to_string(),
                "Thumb Clockwise".to_string(),
                "Hat Top".to_string(),
                "Hat Left".to_string(),
                "Hat Right".to_string(),
                "Hat Bottom".to_string(),
                "Button 1".to_string(),
                "Button 2".to_string(),
                "Precision Aim".to_string(),
                "Button 3".to_string(),
            ],
        );
        let mut timer = Timer::new(Duration::from_millis(100));

        loop {
            if let Some((is_running, data)) = client_dualchannel.recv() {
                if is_running {
                    if data.len() == 0 {
                        client_dualchannel.send((true, driver_configuration_descriptor.to_bytes()));

                        update_device_list(&client_dualchannel, device_list_mutex.clone());
                    } else {
                        println!("{:?}", data);
                    }
                }
            }

            if let Some(message) = child.recv() {
                match message {
                    Message::DeviceListUpdate => {
                        update_device_list(&client_dualchannel, device_list_mutex.clone())
                    }
                }
            }

            timer.wait();
        }
    });
}

fn update_device_list(
    client_dualchannel: &DualChannel<(bool, Vec<u8>)>,
    device_list_mutex: Arc<Mutex<HashSet<String>>>,
) {
    let mut serial_number_vec = vec![];

    for serial_number in device_list_mutex.lock_safe().iter() {
        serial_number_vec.push(serial_number.clone());
    }

    client_dualchannel.send((true, DeviceList::new(serial_number_vec).to_bytes()));
}
