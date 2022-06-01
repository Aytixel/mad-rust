mod window;

use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::*;
use webrender::api::*;
use window::ext::*;
use window::{Event, *};
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::*;

fn main() {
    let mut window_options = WindowOptions::new("Mad rust", 1080, 720, Some("./ui/icon.png"));
    window_options.transparent = true;
    window_options.decorations = false;
    window_options.min_size = Some(PhysicalSize::new(533, 300));

    let mut window = Window::new(
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

#[derive(Clone, FromPrimitive)]
enum AppEvent {
    CloseButton,
    MaximizeButton,
    MinimizeButton,
    TitleBar,
}

impl Into<u64> for AppEvent {
    fn into(self) -> u64 {
        self as u64
    }
}

impl From<u64> for AppEvent {
    fn from(value: u64) -> Self {
        FromPrimitive::from_u64(value).unwrap()
    }
}

struct App {
    font: Font,
    do_render: bool,
    do_exit: bool,
    mouse_position: Option<PhysicalPosition<f64>>,
    event_stack: Vec<(AppEvent, LayoutRect)>,
}

impl App {
    fn new(font: Font) -> Box<Self> {
        Box::new(Self {
            font,
            do_render: true,
            do_exit: false,
            mouse_position: None,
            event_stack: vec![],
        })
    }

    fn calculate_event(&mut self, window: &mut WindowWrapper) -> bool {
        if let Some(mouse_position) = self.mouse_position {
            if let Some(HitTestItem { tag, .. }) = window
                .api
                .hit_test(
                    window.document_id,
                    None,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                    HitTestFlags::empty(),
                )
                .items
                .pop()
            {
                match AppEvent::from(tag.0) {
                    AppEvent::CloseButton => self.do_exit = true,
                    AppEvent::MaximizeButton => window
                        .context
                        .window()
                        .set_maximized(!window.context.window().is_maximized()),
                    AppEvent::MinimizeButton => window.context.window().set_minimized(true),
                    AppEvent::TitleBar => window.context.window().drag_window().unwrap(),
                }

                return true;
            }
        }

        false
    }

    fn draw_title_bar(&mut self, window_size: PhysicalSize<u32>, frame_builder: &mut FrameBuilder) {
        let builder = &mut frame_builder.builder;

        // title bar
        let title_bar_layout_rect = LayoutRect::new(
            LayoutPoint::new(10.0, 10.0),
            LayoutSize::new(window_size.width as f32 - 20.0, 35.0),
        );

        builder.push_rounded_rect(
            &CommonItemProperties::new(title_bar_layout_rect, frame_builder.space_and_clip)
                .add_item_tag((AppEvent::TitleBar.into(), 0)),
            ColorF::from(ColorU::new(66, 66, 66, 100)),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );

        // close button
        let close_button_layout_rect = LayoutRect::new(
            LayoutPoint::new(window_size.width as f32 - 55.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );

        builder.push_rounded_rect(
            &CommonItemProperties::new(close_button_layout_rect, frame_builder.space_and_clip)
                .add_item_tag((AppEvent::CloseButton.into(), 0)),
            ColorF::from(ColorU::new(255, 79, 0, 100)),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );

        // maximize button
        let maximize_button_layout_rect = LayoutRect::new(
            LayoutPoint::new(window_size.width as f32 - 100.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );

        builder.push_rounded_rect(
            &CommonItemProperties::new(maximize_button_layout_rect, frame_builder.space_and_clip)
                .add_item_tag((AppEvent::MaximizeButton.into(), 0)),
            ColorF::from(ColorU::new(255, 189, 0, 100)),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );

        // minimize button
        let minimize_button_layout_rect = LayoutRect::new(
            LayoutPoint::new(window_size.width as f32 - 145.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );

        builder.push_rounded_rect(
            &CommonItemProperties::new(minimize_button_layout_rect, frame_builder.space_and_clip)
                .add_item_tag((AppEvent::MinimizeButton.into(), 0)),
            ColorF::from(ColorU::new(50, 221, 23, 100)),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );

        // pushing events to the event stack
        self.event_stack
            .push((AppEvent::CloseButton, close_button_layout_rect));
        self.event_stack
            .push((AppEvent::MaximizeButton, maximize_button_layout_rect));
        self.event_stack
            .push((AppEvent::MinimizeButton, minimize_button_layout_rect));
        self.event_stack
            .push((AppEvent::TitleBar, title_bar_layout_rect));
    }
}

impl WindowTrait for App {
    fn on_event(&mut self, event: Event, window: &mut WindowWrapper) {
        match event {
            Event::MousePressed(MouseButton::Left) => {
                self.calculate_event(window);
            }
            Event::MousePosition(position) => {
                self.mouse_position = Some(position);
            }
            _ => {}
        }
    }

    fn should_exit(&self) -> bool {
        self.do_exit
    }

    fn should_rerender(&self) -> bool {
        self.do_render
    }

    fn render(&mut self, frame_builder: &mut FrameBuilder, window: &mut WindowWrapper) {
        self.event_stack.clear();

        let window_size = window.get_window_size();

        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min(),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_title_bar(window_size, frame_builder);

        frame_builder.builder.pop_stacking_context();

        self.do_render = false;
    }
}
