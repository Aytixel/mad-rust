// hide the console on release builds for windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod animation;
mod connection;
mod ui;
mod window;

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use connection::Connection;
use ui::{App, DocumentTrait};

use hashbrown::HashMap;
use util::connection::command::DeviceConfig;
use util::thread::MutexTrait;
use util::{
    connection::command::{DeviceList, DriverConfigurationDescriptor},
    thread::kill_double,
};
use window::{Font, GlobalStateTrait, Window, WindowOptions};
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::PhysicalSize;

pub struct Driver {
    driver_configuration_descriptor: DriverConfigurationDescriptor,
    device_list: DeviceList,
}

impl Driver {
    fn new(driver_configuration_descriptor: DriverConfigurationDescriptor) -> Self {
        Self {
            driver_configuration_descriptor,
            device_list: DeviceList::default(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DeviceId {
    socket_addr: SocketAddr,
    serial_number: String,
}

impl DeviceId {
    fn new(socket_addr: SocketAddr, serial_number: String) -> Self {
        Self {
            socket_addr,
            serial_number,
        }
    }
}

enum ConnectionEvent {
    RequestDeviceConfig(DeviceId),
    ApplyDeviceConfig(SocketAddr, DeviceConfig),
}

pub struct GlobalState {
    font_hashmap_mutex: Mutex<HashMap<&'static str, Font>>,
    do_redraw: AtomicBool,
    driver_hashmap_mutex: Mutex<HashMap<SocketAddr, Driver>>,
    device_id_vec_mutex: Mutex<Vec<DeviceId>>,
    selected_device_id_option_mutex: Mutex<Option<DeviceId>>,
    selected_device_config_option_mutex: Mutex<Option<DeviceConfig>>,
    connection_event_queue_mutex: Mutex<VecDeque<ConnectionEvent>>,
    new_document_option_mutex: Mutex<Option<Box<dyn DocumentTrait + Send>>>,
}

impl GlobalState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            font_hashmap_mutex: Mutex::new(HashMap::new()),
            do_redraw: AtomicBool::new(true),
            driver_hashmap_mutex: Mutex::new(HashMap::new()),
            device_id_vec_mutex: Mutex::new(vec![]),
            selected_device_id_option_mutex: Mutex::new(None),
            selected_device_config_option_mutex: Mutex::new(None),
            connection_event_queue_mutex: Mutex::new(VecDeque::new()),
            new_document_option_mutex: Mutex::new(None),
        })
    }

    fn push_connection_event(&self, event: ConnectionEvent) {
        self.connection_event_queue_mutex
            .lock_poisoned()
            .push_back(event);
    }

    fn pop_connection_event(&self) -> Option<ConnectionEvent> {
        self.connection_event_queue_mutex
            .lock_poisoned()
            .pop_front()
    }
}

impl GlobalStateTrait for GlobalState {
    fn should_redraw(&self) -> bool {
        self.do_redraw.swap(false, Ordering::Relaxed)
    }

    fn request_redraw(&self) {
        self.do_redraw.store(true, Ordering::Relaxed);
    }
}

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "windows"))]
    sudo::escalate_if_needed().unwrap();

    if kill_double() {
        return;
    }

    let global_state = GlobalState::new();
    let connection = Connection::new(global_state.clone()).await;

    connection.run().await;

    let mut window_options =
        WindowOptions::new("Mad rust", 1080, 720, include_bytes!("../ui/icon.png"));

    window_options.transparent = true;
    window_options.decorations = false;
    window_options.min_size = Some(PhysicalSize::new(533, 300));

    let mut window = Window::new(window_options, global_state);

    {
        // add background blur effect on windows and macos
        #[cfg(target_os = "windows")]
        apply_blur(&window.wrapper.context.window(), None).ok();

        #[cfg(target_os = "macos")]
        apply_vibrancy(
            &window.context.window(),
            NSVisualEffectMaterial::AppearanceBased,
        )
        .ok();
    }

    window
        .wrapper
        .load_font_file("OpenSans", include_bytes!("../ui/font/OpenSans.ttf"));
    window.set_window::<App>();
    window.run();
    window.deinit();
}
