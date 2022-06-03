use std::collections::HashSet;
use std::sync::Arc;
use std::vec;

use crate::window::ext::*;
use crate::window::{Event, Font, FrameBuilder, WindowInitTrait, WindowTrait, WindowWrapper};
use crate::GlobalState;

use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize, WorldPoint};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, ColorU, CommonItemProperties, DynamicProperties,
    PrimitiveFlags, PropertyBinding, PropertyBindingKey, PropertyValue, RenderReasons,
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
    do_render: bool,
    do_exit: bool,
    mouse_position: Option<PhysicalPosition<f64>>,
    over_states: HashSet<AppEvent>,
    global_state: Arc<GlobalState>,

    close_button_color_key: PropertyBindingKey<ColorF>,
    maximize_button_color_key: PropertyBindingKey<ColorF>,
    minimize_button_color_key: PropertyBindingKey<ColorF>,
}

impl App {
    fn calculate_event(&mut self, window: &mut WindowWrapper, target_event_type: AppEventType) {
        if let Some(mouse_position) = self.mouse_position {
            let hit_items = window
                .api
                .borrow()
                .hit_test(
                    window.document_id,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                )
                .items;

            self.over_states.clear();

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
                if let AppEvent::CloseButton | AppEvent::MaximizeButton | AppEvent::MinimizeButton =
                    event
                {
                    self.over_states.insert(event);
                }
            }

            let mut txn = Transaction::new();

            txn.reset_dynamic_properties();
            txn.append_dynamic_properties(DynamicProperties {
                transforms: vec![],
                floats: vec![],
                colors: vec![
                    PropertyValue {
                        key: self.close_button_color_key,
                        value: ColorF::from(ColorU::new(
                            255,
                            79,
                            0,
                            if self.over_states.contains(&AppEvent::CloseButton) {
                                150
                            } else {
                                100
                            },
                        )),
                    },
                    PropertyValue {
                        key: self.maximize_button_color_key,
                        value: ColorF::from(ColorU::new(
                            255,
                            189,
                            0,
                            if self.over_states.contains(&AppEvent::MaximizeButton) {
                                150
                            } else {
                                100
                            },
                        )),
                    },
                    PropertyValue {
                        key: self.minimize_button_color_key,
                        value: ColorF::from(ColorU::new(
                            50,
                            221,
                            23,
                            if self.over_states.contains(&AppEvent::MinimizeButton) {
                                150
                            } else {
                                100
                            },
                        )),
                    },
                ],
            });
            txn.generate_frame(0, RenderReasons::empty());
            window
                .api
                .borrow_mut()
                .send_transaction(window.document_id, txn);
        }
    }

    fn draw_title_bar(
        &mut self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
        window: &mut WindowWrapper,
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
            ColorF::from(ColorU::new(66, 66, 66, 100)),
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
            &window.api.borrow(),
            "Device List",
            ColorF::from(ColorU::new(255, 255, 255, 100)),
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
                ColorF::from(ColorU::new(255, 79, 0, 100)),
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
                ColorF::from(ColorU::new(255, 189, 0, 100)),
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
                ColorF::from(ColorU::new(50, 221, 23, 100)),
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
    fn new(window: &mut WindowWrapper, global_state: Arc<GlobalState>) -> Box<dyn WindowTrait> {
        Box::new(Self {
            font: window.load_font("OpenSans", Au::from_f32_px(15.0)),
            do_render: true,
            do_exit: false,
            mouse_position: None,
            over_states: HashSet::new(),
            global_state,
            close_button_color_key: PropertyBindingKey::new(0),
            maximize_button_color_key: PropertyBindingKey::new(1),
            minimize_button_color_key: PropertyBindingKey::new(2),
        })
    }
}

impl WindowTrait for App {
    fn on_event(&mut self, event: Event, window: &mut WindowWrapper) {
        match event {
            Event::Resized(_) | Event::MouseEntered | Event::MouseLeft => {
                self.calculate_event(window, AppEventType::UpdateOverState);
            }
            Event::MousePressed(MouseButton::Left) => {
                self.calculate_event(window, AppEventType::MousePressed);
            }
            Event::MouseReleased(MouseButton::Left) => {
                self.calculate_event(window, AppEventType::MouseReleased);
            }
            Event::MousePosition(position) => {
                self.calculate_event(window, AppEventType::UpdateOverState);
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
            frame_builder.bounds.min,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        self.draw_title_bar(window_size, frame_builder, window);

        frame_builder.builder.pop_stacking_context();

        self.do_render = false;
    }

    fn unload(&mut self) {
        self.font.unload();
    }
}
