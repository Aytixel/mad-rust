mod animation;
mod connection;
mod ui;
mod window;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::ThreadId;

use connection::Connection;
use ui::App;
use window::{GlobalStateTrait, Window, WindowOptions};

use util::{
    connection::command::{DeviceConfigurationDescriptor, DeviceList},
    thread::kill_double,
};
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::PhysicalSize;

pub struct Driver {
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

pub struct GlobalState {
    do_redraw: AtomicBool,
    driver_hashmap_mutex: Mutex<HashMap<ThreadId, Driver>>,
}

impl GlobalState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            do_redraw: AtomicBool::new(true),
            driver_hashmap_mutex: Mutex::new(HashMap::new()),
        })
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
