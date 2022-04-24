/*
VID     PID
0x0738  0x1713
*/
use std::time::Duration;

use rusb::{open_device_with_vid_pid, DeviceHandle, UsbContext};

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

fn main() {
    let mut device_handle = open_device_with_vid_pid(0x0738, 0x1713).unwrap();

    run(&mut device_handle);
}

fn run<T: UsbContext>(device_handle: &mut DeviceHandle<T>) {
    let device = device_handle.device();
    let device_descriptor = device.device_descriptor().unwrap();
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

    println!("{:?}", endpoint);

    println!(
        "Bus {:03} Device {:03} ID {:04x}:{:04x}",
        device.bus_number(),
        device.address(),
        device_descriptor.vendor_id(),
        device_descriptor.product_id()
    );

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
    let timeout = Duration::from_secs(1);

    loop {
        match device_handle.read_interrupt(endpoint.address, &mut buf, timeout) {
            Ok(len) => {
                println!("{:?}", &buf[..len]);
            }
            Err(rusb::Error::Timeout)
            | Err(rusb::Error::Pipe)
            | Err(rusb::Error::Overflow)
            | Err(rusb::Error::Io) => {}
            Err(err) => {
                println!("could not read from endpoint: {}", err);
                break;
            }
        }
    }

    if has_kernel_driver {
        device_handle.attach_kernel_driver(endpoint.iface).ok();
    }
}
