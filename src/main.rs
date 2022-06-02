mod animation;
mod ui;
mod window;

use ui::App;
use window::{Window, WindowOptions};

use webrender::api::{ColorF, ColorU};
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::PhysicalSize;

pub struct GlobalState {}

impl GlobalState {
    fn new() -> Self {
        Self {}
    }
}

fn main() {
    let mut window_options = WindowOptions::new("Mad rust", 1080, 720, Some("./ui/icon.png"));

    window_options.transparent = true;
    window_options.decorations = false;
    window_options.min_size = Some(PhysicalSize::new(533, 300));

    let mut window = Window::new(
        window_options,
        GlobalState::new(),
        ColorF::from(ColorU::new(33, 33, 33, 240)),
    );

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
        .load_font_file("OpenSans", "./ui/font/OpenSans.ttf");
    window.set_window::<App>();
    window.run();
    window.deinit();
}
