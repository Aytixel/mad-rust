mod title_bar;
mod window_resize;

use std::collections::HashSet;
use std::sync::Arc;

use crate::animation::Animation;
use crate::window::ext::ColorFTrait;
use crate::window::{Event, Font, FrameBuilder, WindowInitTrait, WindowTrait, WindowWrapper};
use crate::GlobalState;

use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::{Au, WorldPoint};
use webrender::api::{ColorF, PrimitiveFlags, PropertyBindingKey};
use webrender::Transaction;
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;

#[derive(Clone, PartialEq, Eq, Hash, FromPrimitive, Debug)]
pub enum AppEvent {
    WindowResizeTopLeft,
    WindowResizeTopRight,
    WindowResizeTop,
    WindowResizeBottomLeft,
    WindowResizeBottomRight,
    WindowResizeBottom,
    WindowResizeLeft,
    WindowResizeRight,
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

#[derive(Clone, Debug)]
pub enum AppEventType {
    MousePressed,
    MouseReleased,
    UpdateOverState,
}

pub struct App {
    font: Font,
    do_exit: bool,
    do_redraw: bool,
    mouse_position: Option<PhysicalPosition<f64>>,
    over_states: HashSet<AppEvent>,
    global_state: Arc<GlobalState>,
    close_button_color_key: PropertyBindingKey<ColorF>,
    maximize_button_color_key: PropertyBindingKey<ColorF>,
    minimize_button_color_key: PropertyBindingKey<ColorF>,
    close_button_color_animation: Animation<ColorF>,
    maximize_button_color_animation: Animation<ColorF>,
    minimize_button_color_animation: Animation<ColorF>,
    resizing: Option<AppEvent>,
}

impl App {
    fn calculate_event(&mut self, wrapper: &mut WindowWrapper, target_event_type: AppEventType) {
        if let Some(mouse_position) = self.mouse_position {
            let hit_items = wrapper
                .api
                .borrow()
                .hit_test(
                    wrapper.document_id,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                )
                .items;

            // event processing
            let event = AppEvent::from(hit_items[0].tag.0);

            match target_event_type {
                AppEventType::MousePressed => match event {
                    AppEvent::TitleBar => wrapper.context.window().drag_window().unwrap(),
                    AppEvent::WindowResizeTopLeft
                    | AppEvent::WindowResizeTopRight
                    | AppEvent::WindowResizeTop
                    | AppEvent::WindowResizeBottomLeft
                    | AppEvent::WindowResizeBottomRight
                    | AppEvent::WindowResizeBottom
                    | AppEvent::WindowResizeLeft
                    | AppEvent::WindowResizeRight => self.resizing = Some(event.clone()),
                    _ => {}
                },
                AppEventType::MouseReleased => match event {
                    AppEvent::CloseButton => self.do_exit = true,
                    AppEvent::MaximizeButton => wrapper
                        .context
                        .window()
                        .set_maximized(!wrapper.context.window().is_maximized()),
                    AppEvent::MinimizeButton => wrapper.context.window().set_minimized(true),
                    _ => {}
                },
                _ => {}
            }

            // over states processing
            let mut new_over_state = HashSet::new();

            for hit_item in hit_items {
                let event = AppEvent::from(hit_item.tag.0);

                if let AppEventType::UpdateOverState = target_event_type {
                    if let AppEvent::WindowResizeTopLeft
                    | AppEvent::WindowResizeTopRight
                    | AppEvent::WindowResizeTop
                    | AppEvent::WindowResizeBottomLeft
                    | AppEvent::WindowResizeBottomRight
                    | AppEvent::WindowResizeBottom
                    | AppEvent::WindowResizeLeft
                    | AppEvent::WindowResizeRight
                    | AppEvent::CloseButton
                    | AppEvent::MaximizeButton
                    | AppEvent::MinimizeButton = event
                    {
                        new_over_state.insert(event);
                    }
                }
            }

            if self.over_states != new_over_state {
                self.update_title_bar_over_state(&new_over_state);
            }

            self.update_window_resize_cursor_icon(&new_over_state, wrapper);
            self.over_states = new_over_state;
        }
    }
}

impl WindowInitTrait<GlobalState> for App {
    fn new(wrapper: &mut WindowWrapper, global_state: Arc<GlobalState>) -> Box<dyn WindowTrait> {
        let over_color_animation = |from: &ColorF, to: &ColorF, value: &mut ColorF, coef: f64| {
            value.a = (to.a - from.a) * coef as f32 + from.a
        };

        Box::new(Self {
            font: wrapper.load_font("OpenSans", Au::from_f32_px(15.0)),
            do_exit: false,
            do_redraw: true,
            mouse_position: None,
            over_states: HashSet::new(),
            global_state,
            close_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            maximize_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            minimize_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            close_button_color_animation: Animation::new(
                ColorF::new_u(255, 79, 0, 100),
                over_color_animation,
            ),
            maximize_button_color_animation: Animation::new(
                ColorF::new_u(255, 189, 0, 100),
                over_color_animation,
            ),
            minimize_button_color_animation: Animation::new(
                ColorF::new_u(50, 221, 23, 100),
                over_color_animation,
            ),
            resizing: None,
        })
    }
}

impl WindowTrait for App {
    fn on_event(&mut self, event: Event, wrapper: &mut WindowWrapper) {
        match event {
            Event::Resized(_) | Event::MouseEntered | Event::MouseLeft => {
                self.calculate_event(wrapper, AppEventType::UpdateOverState);
            }
            Event::MousePressed(MouseButton::Left) => {
                self.calculate_event(wrapper, AppEventType::MousePressed);
            }
            Event::MouseReleased(MouseButton::Left) => {
                self.calculate_event(wrapper, AppEventType::MouseReleased);
            }
            Event::MousePosition(position) => {
                self.mouse_position = Some(position);
                self.calculate_event(wrapper, AppEventType::UpdateOverState);
            }
            Event::DeviceMotion(delta) => {
                self.update_window_resize(delta, wrapper);
            }
            Event::DeviceReleased(button) => {
                // mouse left button
                if button == 1 {
                    self.resizing = None;

                    self.calculate_event(wrapper, AppEventType::UpdateOverState);
                }
            }
            _ => {}
        }
    }

    fn should_exit(&self) -> bool {
        self.do_exit
    }

    fn should_redraw(&mut self) -> bool {
        let value = self.do_redraw;

        self.do_redraw = false;

        value
    }

    fn animate(&mut self, txn: &mut Transaction) {
        self.animate_title_bar(txn);
    }

    fn redraw(&mut self, frame_builder: &mut FrameBuilder, wrapper: &mut WindowWrapper) {
        let window_size = wrapper.get_window_size();

        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_title_bar(window_size, frame_builder, wrapper);
        self.draw_window_resize(window_size, frame_builder);

        frame_builder.builder.pop_stacking_context();
    }

    fn unload(&mut self) {
        self.font.unload();
    }
}
