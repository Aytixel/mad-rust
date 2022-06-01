use std::collections::HashSet;
use std::sync::Arc;

use crate::window::ext::*;
use crate::window::{Event, Font, FrameBuilder, WindowInitTrait, WindowTrait, WindowWrapper};
use crate::GlobalState;

use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize, WorldPoint};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, ColorU, CommonItemProperties, HitTestFlags, PrimitiveFlags,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::MouseButton;

#[derive(Clone, PartialEq, Eq, Hash, FromPrimitive)]
enum AppEvent {
    CloseButton,
    MaximizeButton,
    MinimizeButton,
    TitleBar,
}

impl AppEvent {
    fn into(self) -> u64 {
        self as u64
    }

    fn from(value: u64) -> Self {
        FromPrimitive::from_u64(value).unwrap()
    }
}

#[derive(Clone)]
enum AppEventType {
    MousePressed,
    MouseReleased,
    MousePosition, // trigger over states
}

pub struct App {
    font: Font,
    do_render: bool,
    do_exit: bool,
    mouse_position: Option<PhysicalPosition<f64>>,
    over_states: HashSet<AppEvent>,
    new_over_states: HashSet<AppEvent>,
    global_state: Arc<GlobalState>,
}

impl App {
    fn update_over_state(&mut self) {
        self.do_render = self.do_render || !self.over_states.is_subset(&self.new_over_states);
        self.over_states = self.new_over_states.clone();
        self.new_over_states.clear();
    }

    fn set_over_state(&mut self, event: AppEvent) {
        self.do_render = true;
        self.new_over_states.insert(event);
    }

    fn calculate_event(
        &mut self,
        window: &mut WindowWrapper,
        target_event_type: AppEventType,
    ) -> bool {
        if let Some(mouse_position) = self.mouse_position {
            let hit_items = window
                .api
                .hit_test(
                    window.document_id,
                    None,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                    HitTestFlags::FIND_ALL,
                )
                .items;

            self.update_over_state();

            for (index, hit_item) in hit_items.iter().enumerate() {
                let event = AppEvent::from(hit_item.tag.0);

                if index == 0 {
                    match target_event_type {
                        AppEventType::MousePressed => match event {
                            AppEvent::TitleBar => window.context.window().drag_window().unwrap(),
                            _ => {}
                        },
                        AppEventType::MouseReleased => match event {
                            AppEvent::CloseButton => self.do_exit = true,
                            AppEvent::MaximizeButton => window
                                .context
                                .window()
                                .set_maximized(!window.context.window().is_maximized()),
                            AppEvent::MinimizeButton => window.context.window().set_minimized(true),
                            _ => {}
                        },
                        _ => {}
                    }
                }

                // over states processing
                match event {
                    AppEvent::CloseButton => self.set_over_state(AppEvent::CloseButton),
                    AppEvent::MaximizeButton => self.set_over_state(AppEvent::MaximizeButton),
                    AppEvent::MinimizeButton => self.set_over_state(AppEvent::MinimizeButton),
                    _ => {}
                }
            }

            return hit_items.len() > 0;
        }

        false
    }

    fn draw_title_bar(
        &mut self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
        window: &mut WindowWrapper,
    ) {
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

        // title
        self.font.push_text(
            builder,
            &window.api,
            "Device List",
            ColorF::from(ColorU::new(255, 255, 255, 150)),
            LayoutPoint::new(20.0, 17.0),
            frame_builder.space_and_clip,
            None,
        );

        // close button
        let close_button_layout_rect = LayoutRect::new(
            LayoutPoint::new(window_size.width as f32 - 55.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );

        builder.push_rounded_rect(
            &CommonItemProperties::new(close_button_layout_rect, frame_builder.space_and_clip)
                .add_item_tag((AppEvent::CloseButton.into(), 0)),
            ColorF::from(ColorU::new(
                255,
                79,
                0,
                if self.over_states.contains(&AppEvent::CloseButton) {
                    150
                } else {
                    100
                },
            )),
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
            ColorF::from(ColorU::new(
                255,
                189,
                0,
                if self.over_states.contains(&AppEvent::MaximizeButton) {
                    150
                } else {
                    100
                },
            )),
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
            ColorF::from(ColorU::new(
                50,
                221,
                23,
                if self.over_states.contains(&AppEvent::MinimizeButton) {
                    150
                } else {
                    100
                },
            )),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );
    }
}

impl WindowInitTrait<GlobalState> for App {
    fn new(window: &mut WindowWrapper, global_state: Arc<GlobalState>) -> Box<dyn WindowTrait> {
        Box::new(Self {
            font: window.load_font("OpenSans", Au::from_f32_px(15.0)),
            do_render: true,
            do_exit: false,
            mouse_position: None,
            over_states: HashSet::new(),
            new_over_states: HashSet::new(),
            global_state,
        })
    }
}

impl WindowTrait for App {
    fn on_event(&mut self, event: Event, window: &mut WindowWrapper) {
        match event {
            Event::MousePressed(MouseButton::Left) => {
                self.calculate_event(window, AppEventType::MousePressed);
            }
            Event::MouseReleased(MouseButton::Left) => {
                self.calculate_event(window, AppEventType::MouseReleased);
            }
            Event::MousePosition(position) => {
                self.calculate_event(window, AppEventType::MousePosition);
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
        let window_size = window.get_window_size();

        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min(),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_title_bar(window_size, frame_builder, window);

        frame_builder.builder.pop_stacking_context();

        self.do_render = false;
    }

    fn unload(&self) {
        self.font.unload();
    }
}
