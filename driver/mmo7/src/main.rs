use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use rusb::{Context, DeviceHandle, UsbContext};

const TIMEOUT: Duration = Duration::from_secs(1);
const VID: u16 = 0x0738;
const PID: u16 = 0x1713;

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

#[derive(Debug)]
struct Timer {
    frame_duration: Duration,
    last_update: Instant,
}

impl Timer {
    fn new(frame_duration: Duration) -> Self {
        Self {
            frame_duration: frame_duration,
            last_update: Instant::now(),
        }
    }

    fn wait(&mut self) {
        let elapsed = self.last_update.elapsed();

        if elapsed < self.frame_duration {
            sleep(self.frame_duration - elapsed);
        }

        self.last_update = Instant::now()
    }
}

fn main() {
    let context = Context::new().unwrap();
    let mut device_list: HashMap<String, Arc<AtomicBool>> = HashMap::new();
    let mut timer = Timer::new(TIMEOUT);

    loop {
        for (serial_number, is_running) in device_list.clone().iter() {
            if !(*is_running).load(Ordering::Relaxed) {
                device_list.remove(serial_number);
            }
        }

        for device in context.devices().unwrap().iter() {
            let device_descriptor = device.device_descriptor().unwrap();

            if device_descriptor.vendor_id() == VID && device_descriptor.product_id() == PID {
                if let Ok(device_handle) = device.open() {
                    if let Some(serial_number) = device_handle
                        .read_serial_number_string(
                            device_handle.read_languages(TIMEOUT).unwrap()[0],
                            &device_descriptor,
                            TIMEOUT,
                        )
                        .ok()
                    {
                        if let None = device_list.get(&serial_number) {
                            let is_running = Arc::new(AtomicBool::new(true));

                            device_list.insert(serial_number.clone(), is_running.clone());

                            spawn(move || {
                                run_device(serial_number);

                                (*is_running).store(false, Ordering::Relaxed);
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
                    device_handle.read_languages(TIMEOUT).unwrap()[0],
                    &device_descriptor,
                    TIMEOUT,
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

fn run_device(serial_number: String) {
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

        println!("{} connected", serial_number);

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

        let mut buf = [0; 8];
        let mut timer = Timer::new(Duration::from_micros(500));

        loop {
            match device_handle.read_interrupt(endpoint.address, &mut buf, TIMEOUT) {
                Ok(len) => {
                    println!("{} : {:?}", serial_number, &buf[..len]);
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

            timer.wait();
        }

        if has_kernel_driver {
            device_handle.attach_kernel_driver(endpoint.iface).ok();
        }
    }
}
