mod app;
mod device_list;

use std::collections::HashSet;

use crate::animation::Animation;
use crate::window::ext::ColorFTrait;
use crate::window::{Event, Font, FrameBuilder, WindowInitTrait, WindowTrait, WindowWrapper};
use crate::GlobalState;

use glutin::dpi::PhysicalSize;
use num::FromPrimitive;
use num_derive::FromPrimitive;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize, LayoutVector2D, WorldPoint};
use webrender::api::{
    APZScrollGeneration, ColorF, CommonItemProperties, ExternalScrollId, HasScrollLinkedEffect,
    PipelineId, PrimitiveFlags, PropertyBindingKey, RenderReasons, SampledScrollOffset,
    SpaceAndClipInfo, SpatialTreeItemKey,
};
use webrender::Transaction;
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;

use self::device_list::DeviceList;

const EXT_SCROLL_ID_ROOT: u64 = 0;

#[derive(Clone, PartialEq, Eq, Hash, FromPrimitive, Debug)]
pub enum AppEvent {
    Scroll,
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

    fn from(value: u64) -> Option<Self> {
        FromPrimitive::from_u64(value)
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
    mouse_position: Option<PhysicalPosition<f64>>,
    window_size: PhysicalSize<u32>,
    over_states: HashSet<AppEvent>,
    close_button_color_key: PropertyBindingKey<ColorF>,
    maximize_button_color_key: PropertyBindingKey<ColorF>,
    minimize_button_color_key: PropertyBindingKey<ColorF>,
    close_button_color_animation: Animation<ColorF>,
    maximize_button_color_animation: Animation<ColorF>,
    minimize_button_color_animation: Animation<ColorF>,
    scroll_offset: LayoutVector2D,
    scroll_frame_size: LayoutSize,
    scroll_content_size: LayoutSize,
    resizing: Option<AppEvent>,
    document: Box<dyn DocumentTrait>,
}

impl App {
    fn calculate_event(
        &mut self,
        wrapper: &mut WindowWrapper<GlobalState>,
        target_event_type: AppEventType,
    ) {
        if let Some(mouse_position) = self.mouse_position {
            let hit_items = wrapper
                .api
                .borrow()
                .hit_test(
                    wrapper.document_id,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                )
                .items;

            if !hit_items.is_empty() {
                // event processing
                if let Some(event) = AppEvent::from(hit_items[0].tag.0) {
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
                            AppEvent::MinimizeButton => {
                                wrapper.context.window().set_minimized(true)
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            // over states processing
            let mut new_over_state = HashSet::new();

            for hit_item in hit_items {
                if let Some(event) = AppEvent::from(hit_item.tag.0) {
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
            }

            if self.over_states != new_over_state {
                self.update_title_bar_over_state(&new_over_state);
            }

            self.update_window_resize_cursor_icon(&new_over_state, wrapper);
            self.over_states = new_over_state;
        }
    }

    fn calculate_wheel_scroll(
        &mut self,
        delta: PhysicalPosition<f64>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        if let Some(mouse_position) = self.mouse_position {
            let hit_items = wrapper
                .api
                .borrow()
                .hit_test(
                    wrapper.document_id,
                    WorldPoint::new(mouse_position.x as f32, mouse_position.y as f32),
                )
                .items;

            for hit_item in hit_items {
                if let Some(AppEvent::Scroll) = AppEvent::from(hit_item.tag.0) {
                    if hit_item.tag.1 == EXT_SCROLL_ID_ROOT as u16 {
                        self.scroll_offset = LayoutVector2D::new(
                            (self.scroll_offset.x - delta.x as f32).max(0.0).min(
                                (self.scroll_content_size.width - self.scroll_frame_size.width)
                                    .max(0.0),
                            ),
                            (self.scroll_offset.y - delta.y as f32).max(0.0).min(
                                (self.scroll_content_size.height - self.scroll_frame_size.height)
                                    .max(0.0),
                            ),
                        );

                        let mut txn = Transaction::new();

                        txn.set_scroll_offsets(
                            ExternalScrollId(EXT_SCROLL_ID_ROOT, PipelineId::dummy()),
                            vec![SampledScrollOffset {
                                offset: self.scroll_offset,
                                generation: APZScrollGeneration::default(),
                            }],
                        );
                        txn.generate_frame(0, RenderReasons::empty());
                        wrapper
                            .api
                            .borrow_mut()
                            .send_transaction(wrapper.document_id, txn);

                        break;
                    }
                }
            }
        }
    }
}

impl WindowInitTrait<GlobalState> for App {
    fn new(wrapper: &mut WindowWrapper<GlobalState>) -> Box<dyn WindowTrait<GlobalState>> {
        let over_color_animation = |from: &ColorF, to: &ColorF, value: &mut ColorF, coef: f64| {
            value.a = (to.a - from.a) * coef as f32 + from.a
        };
        let window_size = wrapper.get_window_size();

        Box::new(Self {
            font: wrapper.load_font("OpenSans", Au::from_f32_px(15.0)),
            do_exit: false,
            mouse_position: None,
            window_size: PhysicalSize::new(0, 0),
            over_states: HashSet::new(),
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
            scroll_offset: LayoutVector2D::zero(),
            scroll_frame_size: LayoutSize::new(
                window_size.width as f32 - 20.0,
                window_size.height as f32 - 65.0,
            ),
            scroll_content_size: LayoutSize::zero(),
            resizing: None,
            document: Box::new(DeviceList::new()),
        })
    }
}

impl WindowTrait<GlobalState> for App {
    fn on_event(&mut self, event: Event, wrapper: &mut WindowWrapper<GlobalState>) {
        match event {
            Event::Resized(size) => {
                self.window_size = size;
                self.scroll_frame_size =
                    LayoutSize::new(size.width as f32 - 20.0, size.height as f32 - 65.0);

                self.calculate_event(wrapper, AppEventType::UpdateOverState);
            }
            Event::MouseEntered | Event::MouseLeft => {
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
            Event::MouseWheel(delta) => {
                self.calculate_wheel_scroll(delta, wrapper);
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

    fn animate(&mut self, txn: &mut Transaction) {
        self.animate_title_bar(txn);
        self.document.animate(txn);
    }

    fn redraw(
        &mut self,
        frame_builder: &mut FrameBuilder,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        // calcultate the scroll frame content size
        self.scroll_content_size = self
            .document
            .calculate_size(self.scroll_frame_size, wrapper);

        // scroll frame / main frame
        frame_builder.builder.push_simple_stacking_context(
            LayoutPoint::new(10.0, 55.0),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );
        frame_builder.builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_size(self.scroll_frame_size),
                frame_builder.space_and_clip,
            ),
            (AppEvent::Scroll.into(), EXT_SCROLL_ID_ROOT as u16),
        );

        let spatial_id = frame_builder.builder.define_scroll_frame(
            frame_builder.space_and_clip.spatial_id,
            ExternalScrollId(EXT_SCROLL_ID_ROOT, PipelineId::dummy()),
            LayoutRect::from_size(self.scroll_content_size),
            LayoutRect::from_size(self.scroll_frame_size),
            LayoutVector2D::zero(),
            APZScrollGeneration::default(),
            HasScrollLinkedEffect::No,
            SpatialTreeItemKey::new(0, 0),
        );
        let clip_id = frame_builder.builder.define_clip_rect(
            &frame_builder.space_and_clip,
            LayoutRect::from_size(self.scroll_frame_size),
        );
        let space_and_clip = SpaceAndClipInfo {
            spatial_id,
            clip_id,
        };

        // draw the scroll frame content
        self.document.draw(
            self.scroll_frame_size,
            frame_builder,
            space_and_clip,
            wrapper,
        );

        frame_builder.builder.pop_stacking_context();

        // draw main window elements
        self.draw_title_bar(self.document.get_title(), self.window_size, frame_builder);
        self.draw_window_resize(self.window_size, frame_builder);

        frame_builder.builder.pop_stacking_context();
    }

    fn unload(&mut self) {
        self.font.unload();
    }
}

pub trait DocumentTrait {
    fn get_title(&self) -> &'static str;

    fn animate(&mut self, txn: &mut Transaction);

    fn calculate_size(
        &mut self,
        frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize;

    fn draw(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        wrapper: &mut WindowWrapper<GlobalState>,
    );
}
