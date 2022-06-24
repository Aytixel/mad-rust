// hide the console on release builds for windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod animation;
mod connection;
mod ui;
mod window;

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::ThreadId;

use connection::Connection;
use ui::App;

use hashbrown::HashMap;
use util::connection::command::DeviceConfig;
use util::thread::MutexTrait;
use util::{
    connection::command::{DeviceList, DriverConfigurationDescriptor},
    thread::kill_double,
};
use window::{GlobalStateTrait, Window, WindowOptions};
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
    thread_id: ThreadId,
    serial_number: String,
}

impl DeviceId {
    fn new(thread_id: ThreadId, serial_number: String) -> Self {
        Self {
            thread_id,
            serial_number,
        }
    }
}

enum ConnectionEvent {
    RequestDeviceConfig(DeviceId),
}

pub struct GlobalState {
    do_redraw: AtomicBool,
    driver_hashmap_mutex: Mutex<HashMap<ThreadId, Driver>>,
    device_id_vec_mutex: Mutex<Vec<DeviceId>>,
    selected_device_id_option_mutex: Mutex<Option<DeviceId>>,
    selected_device_config_option_mutex: Mutex<Option<DeviceConfig>>,
    connection_event_queue_mutex: Mutex<VecDeque<ConnectionEvent>>,
}

impl GlobalState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            do_redraw: AtomicBool::new(true),
            driver_hashmap_mutex: Mutex::new(HashMap::new()),
            device_id_vec_mutex: Mutex::new(vec![]),
            selected_device_id_option_mutex: Mutex::new(None),
            selected_device_config_option_mutex: Mutex::new(None),
            connection_event_queue_mutex: Mutex::new(VecDeque::new()),
        })
    }

    fn push_connection_event(&self, event: ConnectionEvent) {
        self.connection_event_queue_mutex
            .lock_safe()
            .push_back(event);
    }

    fn pop_connection_event(&self) -> Option<ConnectionEvent> {
        self.connection_event_queue_mutex.lock_safe().pop_front()
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

fn main() {
    if !kill_double() {
        let global_state = GlobalState::new();
        let connection = Connection::new(global_state.clone());

        connection.run();

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
}
