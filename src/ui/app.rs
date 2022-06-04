use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::vec;

use crate::animation::Animation;
use crate::window::ext::*;
use crate::window::{Event, Font, FrameBuilder, WindowInitTrait, WindowTrait, WindowWrapper};
use crate::GlobalState;

use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize, WorldPoint};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, DynamicProperties, PrimitiveFlags,
    PropertyBinding, PropertyBindingKey, PropertyValue,
};
use webrender::Transaction;
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

            let mut new_over_state = HashSet::new();

            for (index, hit_item) in hit_items.iter().enumerate() {
                let event = AppEvent::from(hit_item.tag.0);

                if index == 0 {
                    match target_event_type {
                        AppEventType::MousePressed => match event {
                            AppEvent::TitleBar => wrapper.context.window().drag_window().unwrap(),
                            _ => {}
                        },
                        AppEventType::MouseReleased => match event {
                            AppEvent::CloseButton => self.do_exit = true,
                            AppEvent::MaximizeButton => wrapper
                                .context
                                .window()
                                .set_maximized(!wrapper.context.window().is_maximized()),
                            AppEvent::MinimizeButton => {
                                wrapper.context.window().set_minimized(true)
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }

                // over states processing
                if let AppEvent::CloseButton | AppEvent::MaximizeButton | AppEvent::MinimizeButton =
                    event
                {
                    new_over_state.insert(event);
                }
            }

            if self.over_states != new_over_state {
                self.close_button_color_animation.to(
                    if new_over_state.contains(&AppEvent::CloseButton) {
                        ColorF::new_u(255, 79, 0, 150)
                    } else {
                        ColorF::new_u(255, 79, 0, 100)
                    },
                    Duration::from_millis(100),
                );
                self.maximize_button_color_animation.to(
                    if new_over_state.contains(&AppEvent::MaximizeButton) {
                        ColorF::new_u(255, 189, 0, 150)
                    } else {
                        ColorF::new_u(255, 189, 0, 100)
                    },
                    Duration::from_millis(100),
                );
                self.minimize_button_color_animation.to(
                    if new_over_state.contains(&AppEvent::MinimizeButton) {
                        ColorF::new_u(50, 221, 23, 150)
                    } else {
                        ColorF::new_u(50, 221, 23, 100)
                    },
                    Duration::from_millis(100),
                );
            }

            self.over_states = new_over_state;
        }
    }

    fn draw_title_bar(
        &mut self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
        wrapper: &mut WindowWrapper,
    ) {
        let builder = &mut frame_builder.builder;

        // title bar
        let title_bar_layout_rect = LayoutRect::new_with_size(
            LayoutPoint::new(10.0, 10.0),
            LayoutSize::new(window_size.width as f32 - 20.0, 35.0),
        );
        let title_bar_common_item_properties =
            &CommonItemProperties::new(title_bar_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect(
            title_bar_common_item_properties,
            ColorF::new_u(66, 66, 66, 100),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            title_bar_common_item_properties,
            (AppEvent::TitleBar.into(), 0),
        );

        // title
        self.font.push_text(
            builder,
            &wrapper.api.borrow(),
            "Device List",
            ColorF::new_u(255, 255, 255, 100),
            LayoutPoint::new(20.0, 17.0),
            frame_builder.space_and_clip,
            None,
        );

        // close button
        let close_button_layout_rect = LayoutRect::new_with_size(
            LayoutPoint::new(window_size.width as f32 - 55.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );
        let close_button_common_item_properties =
            &CommonItemProperties::new(close_button_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect_with_animation(
            &CommonItemProperties::new(close_button_layout_rect, frame_builder.space_and_clip),
            PropertyBinding::Binding(
                self.close_button_color_key,
                self.close_button_color_animation.value,
            ),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            close_button_common_item_properties,
            (AppEvent::CloseButton.into(), 0),
        );

        // maximize button
        let maximize_button_layout_rect = LayoutRect::new_with_size(
            LayoutPoint::new(window_size.width as f32 - 100.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );
        let maximize_button_common_item_properties =
            &CommonItemProperties::new(maximize_button_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect_with_animation(
            maximize_button_common_item_properties,
            PropertyBinding::Binding(
                self.maximize_button_color_key,
                self.maximize_button_color_animation.value,
            ),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            maximize_button_common_item_properties,
            (AppEvent::MaximizeButton.into(), 0),
        );

        // minimize button
        let minimize_button_layout_rect = LayoutRect::new_with_size(
            LayoutPoint::new(window_size.width as f32 - 145.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );
        let minimize_button_common_item_properties =
            &CommonItemProperties::new(minimize_button_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect_with_animation(
            minimize_button_common_item_properties,
            PropertyBinding::Binding(
                self.minimize_button_color_key,
                self.minimize_button_color_animation.value,
            ),
            BorderRadius::new(3.0, 3.0, 3.0, 3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            minimize_button_common_item_properties,
            (AppEvent::MinimizeButton.into(), 0),
        );
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
                self.calculate_event(wrapper, AppEventType::UpdateOverState);
                self.mouse_position = Some(position);
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
        let mut colors = vec![];

        if self.close_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.close_button_color_key,
                value: self.close_button_color_animation.value,
            });
        }
        if self.maximize_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.maximize_button_color_key,
                value: self.maximize_button_color_animation.value,
            });
        }
        if self.minimize_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.minimize_button_color_key,
                value: self.minimize_button_color_animation.value,
            });
        }

        if !colors.is_empty() {
            txn.append_dynamic_properties(DynamicProperties {
                transforms: vec![],
                floats: vec![],
                colors,
            });
        }
    }

    fn redraw(&mut self, frame_builder: &mut FrameBuilder, wrapper: &mut WindowWrapper) {
        let window_size = wrapper.get_window_size();

        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_title_bar(window_size, frame_builder, wrapper);

        frame_builder.builder.pop_stacking_context();
    }

    fn unload(&mut self) {
        self.font.unload();
    }
}
