mod window;

use webrender::api::units::*;
use webrender::api::*;
use window::ext::*;
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::PhysicalSize;

fn main() {
    let mut window_options = window::WindowOptions::new(1080, 720, Some("./ui/icon.png"));
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

    let app = App::new(
        window
            .wrapper
            .load_font("OpenSans", units::Au::from_f32_px(32.0)),
    );

    window.set_window(app);
    window.run();
    window.deinit();
}

struct App {
    font: window::Font,
    do_render: bool,
}

impl App {
    fn new(font: window::Font) -> Box<Self> {
        Box::new(Self {
            font,
            do_render: true,
        })
    }

    fn draw_window_button(
        &self,
        position: LayoutPoint,
        size: LayoutSize,
        color: ColorU,
        frame_builder: &mut window::FrameBuilder,
    ) {
        let builder = &mut frame_builder.builder;
        let edge = 3.0;
        let (width, _) = size.to_tuple();

        builder.push_rounded_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    position - LayoutSize::new(edge, 0.0),
                    LayoutSize::new(edge, edge),
                ),
                frame_builder.space_and_clip,
            ),
            ColorF::from(color),
            BorderRadius::new(0.0, edge, 0.0, 0.0),
            ClipMode::ClipOut,
        );
        builder.push_rounded_rect(
            &CommonItemProperties::new(
                LayoutRect::new(position, size),
                frame_builder.space_and_clip,
            ),
            ColorF::from(color),
            BorderRadius::new(0.0, 0.0, 3.0, 3.0),
            ClipMode::Clip,
        );
        builder.push_rounded_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    position + LayoutSize::new(width, 0.0),
                    LayoutSize::new(edge, edge),
                ),
                frame_builder.space_and_clip,
            ),
            ColorF::from(color),
            BorderRadius::new(edge, 0.0, 0.0, 0.0),
            ClipMode::ClipOut,
        );
    }
}

impl window::WindowTrait for App {
    fn get_title(&self) -> &'static str {
        "Mad rust"
    }

    fn on_event(&mut self, _event: window::Event, _window: &mut window::WindowWrapper) {}

    fn should_rerender(&self) -> bool {
        self.do_render
    }

    fn render(
        &mut self,
        frame_builder: &mut window::FrameBuilder,
        window: &mut window::WindowWrapper,
    ) {
        let window_size = window.get_window_size();

        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min(),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_window_button(
            LayoutPoint::new(window_size.width as f32 - 157.0, 0.0),
            LayoutSize::new(40.0, 30.0),
            ColorU::new(50, 221, 23, 100),
            frame_builder,
        );

        self.draw_window_button(
            LayoutPoint::new(window_size.width as f32 - 106.0, 0.0),
            LayoutSize::new(40.0, 30.0),
            ColorU::new(255, 189, 0, 100),
            frame_builder,
        );

        self.draw_window_button(
            LayoutPoint::new(window_size.width as f32 - 55.0, 0.0),
            LayoutSize::new(40.0, 30.0),
            ColorU::new(255, 79, 0, 100),
            frame_builder,
        );

        frame_builder.builder.pop_stacking_context();

        self.do_render = false;
    }
}
