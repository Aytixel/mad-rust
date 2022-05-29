mod window;

use webrender::api::units::*;
use webrender::api::*;
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::PhysicalSize;

fn main() {
    let mut window_options = window::WindowOptions::new("test", 1080, 720, Some("./ui/icon.png"));
    window_options.transparent = true;
    window_options.decorations = false;
    window_options.min_size = Some(PhysicalSize::new(533, 300));

    let mut window = window::Window::new(
        window_options,
        Some(ColorF::from(ColorU::new(33, 33, 33, 240))),
        0,
    );

    {
        // add background blur effect on windows and macos
        #[cfg(target_os = "windows")]
        apply_blur(&window.context.window(), None).ok();

        #[cfg(target_os = "macos")]
        apply_vibrancy(
            &window.context.window(),
            NSVisualEffectMaterial::AppearanceBased,
        )
        .ok();
    }

    window.load_font_file("OpenSans", "./ui/font/OpenSans.ttf");

    let app = App::new(window.load_font("OpenSans", units::Au::from_f32_px(32.0)));

    window.set_window(app);
    window.run();
    window.deinit();
}

struct App {
    font: window::Font,
    has_rendered: bool,
}

impl App {
    fn new(font: window::Font) -> Box<Self> {
        Box::new(Self {
            font,
            has_rendered: false,
        })
    }
}

impl window::WindowTrait for App {
    fn on_event(&mut self, _: Vec<window::Event>, _: &mut window::Window) {}

    fn should_rerender(&self) -> bool {
        !self.has_rendered
    }

    fn render(&mut self, frame_builder: &mut window::FrameBuilder, window: &mut window::Window) {
        let builder = &mut frame_builder.builder;

        builder.push_simple_stacking_context(
            frame_builder.bounds.min(),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    LayoutPoint::new(100.0, 200.0),
                    LayoutSize::new(100.0, 200.0),
                ),
                frame_builder.space_and_clip,
            ),
            ColorF::new(0.0, 1.0, 0.0, 1.0),
        );

        self.font.push_text(
            frame_builder,
            &window.api,
            "Salut comment\n ça\r\tva",
            ColorF::new(1.0, 1.0, 0.0, 1.0),
            None,
            LayoutPoint::new(100.0, 50.0),
        );

        frame_builder.builder.pop_stacking_context();

        self.has_rendered = true;
    }
}
