mod mapper;

use std::collections::HashMap;
use std::env::current_exe;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;

use mapper::Mapper;
use rusb::{Context, DeviceHandle, UsbContext};
use util::connection::{command::*, Client, CommandTrait};
use util::thread::{kill_double, DualChannel};
use util::time::{Timer, TIMEOUT_1S};

const VID: u16 = 0x0738;
const PID: u16 = 0x1713;

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
        let context = Context::new().unwrap();
        let device_list_mutex = Arc::new(Mutex::new(HashMap::<String, Arc<AtomicBool>>::new()));
        let (host, child) = DualChannel::<Message>::new();

        run_connection(client_dualchannel, child, device_list_mutex.clone());
        listening_new_device(context, host, device_list_mutex);
    }
}

// device handling
fn listening_new_device(
    context: Context,
    host: DualChannel<Message>,
    device_list_mutex: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) {
    let mut timer = Timer::new(TIMEOUT_1S);

    loop {
        let mut device_list = match device_list_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        for device in context.devices().unwrap().iter() {
            let device_descriptor = device.device_descriptor().unwrap();

            if device_descriptor.vendor_id() == VID && device_descriptor.product_id() == PID {
                if let Ok(device_handle) = device.open() {
                    if let Some(serial_number) = device_handle
                        .read_serial_number_string(
                            device_handle.read_languages(TIMEOUT_1S).unwrap()[0],
                            &device_descriptor,
                            TIMEOUT_1S,
                        )
                        .ok()
                    {
                        if let None = device_list.get(&serial_number) {
                            let host = host.clone();
                            let is_running = Arc::new(AtomicBool::new(true));

                            device_list.insert(serial_number.clone(), is_running.clone());

                            spawn(move || {
                                run_device(serial_number, host.clone());

                                (*is_running).store(false, Ordering::Relaxed);

                                host.send(Message::DeviceListUpdate);
                            });
                        }
                    }
                }
            }
        }

        timer.wait();
    }
}

fn find_device(serial_number: String) -> Option<DeviceHandle<Context>> {
    for device in Context::new().unwrap().devices().unwrap().iter() {
        let device_descriptor = device.device_descriptor().unwrap();

        if device_descriptor.vendor_id() == VID && device_descriptor.product_id() == PID {
            let device_handle = device.open().unwrap();

            if let Some(serial_number_found) = device_handle
                .read_serial_number_string(
                    device_handle.read_languages(TIMEOUT_1S).unwrap()[0],
                    &device_descriptor,
                    TIMEOUT_1S,
                )
                .ok()
            {
                if serial_number == serial_number_found {
                    return Some(device_handle);
                }
            }
        }
    }

    None
}

fn run_device(serial_number: String, dual_channel: DualChannel<Message>) {
    if let Some(mut device_handle) = find_device(serial_number.clone()) {
        let device = device_handle.device();
        let config_descriptor = device.config_descriptor(0).unwrap();
        let interface = config_descriptor.interfaces().next().unwrap();
        let interface_descriptor = interface.descriptors().next().unwrap();
        let endpoint_descriptor = interface_descriptor.endpoint_descriptors().next().unwrap();
        let endpoint = Endpoint {
            config: config_descriptor.number(),
            iface: interface_descriptor.interface_number(),
            setting: interface_descriptor.setting_number(),
            address: endpoint_descriptor.address(),
        };

        let has_kernel_driver = match device_handle.kernel_driver_active(endpoint.iface) {
            Ok(true) => {
                device_handle.detach_kernel_driver(endpoint.iface).ok();
                true
            }
            _ => false,
        };

        device_handle
            .set_active_configuration(endpoint.config)
            .unwrap();
        device_handle.claim_interface(endpoint.iface).unwrap();
        device_handle
            .set_alternate_setting(endpoint.iface, endpoint.setting)
            .unwrap();

        println!("{} connected", serial_number);

        dual_channel.send(Message::DeviceListUpdate);

        let mut buffer = [0; 8];
        let mut mapper = Mapper::new();

        loop {
            match device_handle.read_interrupt(
                endpoint.address,
                &mut buffer,
                Duration::from_millis(1),
            ) {
                Ok(_) => {
                    //println!("{} : {:?}", serial_number, buffer);
                    mapper.emulate(&buffer);
                }
                Err(rusb::Error::Timeout)
                | Err(rusb::Error::Pipe)
                | Err(rusb::Error::Overflow)
                | Err(rusb::Error::Io) => {}
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

// connection processing
fn run_connection(
    client_dualchannel: DualChannel<(bool, Vec<u8>)>,
    child: DualChannel<Message>,
    device_list_mutex: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) {
    spawn(move || {
        let mut device_configuration_descriptor = DeviceConfigurationDescriptor::new(
            VID,
            PID,
            "MMO7".to_string(),
            current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join(Path::new("icon.png"))
                .to_str()
                .unwrap()
                .to_string(),
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
        let mut device_list = match device_list_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        for (serial_number, is_running) in device_list.clone().iter() {
            if !(*is_running).load(Ordering::Relaxed) {
                device_list.remove(serial_number);
            }
        }

        loop {
            if let Some((is_running, data)) = client_dualchannel.recv() {
                if is_running {
                    if data.len() == 0 {
                        client_dualchannel.send((true, device_configuration_descriptor.to_bytes()));

                        update_device_list(&client_dualchannel, &device_list);
                    } else {
                        println!("{:?}", data);
                    }
                }
            }

            if let Some(message) = child.recv() {
                match message {
                    Message::DeviceListUpdate => {
                        update_device_list(&client_dualchannel, &device_list)
                    }
                }
            }

            timer.wait();
        }
    });
}

fn update_device_list(
    client_dualchannel: &DualChannel<(bool, Vec<u8>)>,
    device_list: &HashMap<String, Arc<AtomicBool>>,
) {
    let mut serial_number_vec = vec![];

    for (serial_number, is_running) in device_list.iter() {
        if (*is_running).load(Ordering::Relaxed) {
            serial_number_vec.push(serial_number.clone());
        }
    }

    client_dualchannel.send((true, DeviceList::new(serial_number_vec).to_bytes()));
}
