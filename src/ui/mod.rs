mod app;
mod device_configurator;
mod device_list;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, MutexGuard};

use crate::animation::Animation;
use crate::window::ext::ColorFTrait;
use crate::window::{
    Event, Font, FrameBuilder, GlobalStateTrait, WindowInitTrait, WindowTrait, WindowWrapper,
};
use crate::{ConnectionEvent, DeviceId, GlobalState};

use hashbrown::{HashMap, HashSet};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use util::connection::command::DeviceConfig;
use util::thread::MutexTrait;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize, LayoutVector2D};
use webrender::api::{
    APZScrollGeneration, ClipChainId, ColorF, CommonItemProperties, DocumentId, ExternalScrollId,
    HasScrollLinkedEffect, HitTestItem, PipelineId, PrimitiveFlags, PropertyBindingKey,
    RenderReasons, SampledScrollOffset, SpaceAndClipInfo, SpatialTreeItemKey,
};
use webrender::{RenderApi, Transaction};
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;

use self::device_configurator::DeviceConfigurator;
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
    ReturnButton,
    TitleBar,
    ChooseDeviceButton,
}

impl AppEvent {
    fn into(self) -> u64 {
        self as u64
    }

    fn from(value: u64) -> Option<Self> {
        FromPrimitive::from_u64(value)
    }
}

pub enum AppEventType {
    MousePressed,
    MouseReleased,
}

pub struct App {
    font_hashmap: HashMap<&'static str, Font>,
    do_exit: bool,
    over_states: HashSet<AppEvent>,
    close_button_color_key: PropertyBindingKey<ColorF>,
    maximize_button_color_key: PropertyBindingKey<ColorF>,
    minimize_button_color_key: PropertyBindingKey<ColorF>,
    return_button_color_key: PropertyBindingKey<ColorF>,
    close_button_color_animation: Animation<ColorF>,
    maximize_button_color_animation: Animation<ColorF>,
    minimize_button_color_animation: Animation<ColorF>,
    return_button_color_animation: Animation<ColorF>,
    scroll_offset: LayoutVector2D,
    scroll_frame_size: LayoutSize,
    scroll_content_size: LayoutSize,
    resizing: Option<AppEvent>,
    document: Box<dyn DocumentTrait>,
}

impl App {
    fn switch_document(
        &mut self,
        new_document: Box<dyn DocumentTrait>,
        api: Rc<RefCell<RenderApi>>,
        document_id: DocumentId,
        global_state: Arc<GlobalState>,
    ) {
        self.document.unload(api, document_id);
        self.document = new_document;

        global_state.request_redraw();
    }

    fn calculate_event(
        &mut self,
        hit_items: Vec<HitTestItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
        target_event_type: AppEventType,
    ) {
        if !hit_items.is_empty() {
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
                        AppEvent::MinimizeButton => wrapper.context.window().set_minimized(true),
                        AppEvent::ReturnButton => {
                            self.switch_document(
                                Box::new(DeviceList::new()),
                                wrapper.api.clone(),
                                wrapper.document_id,
                                wrapper.global_state.clone(),
                            );

                            let mut selected_device_id_option = wrapper
                                .global_state
                                .selected_device_id_option_mutex
                                .lock_poisoned();
                            let mut selected_device_config_option = wrapper
                                .global_state
                                .selected_device_config_option_mutex
                                .lock_poisoned();

                            *selected_device_id_option = None;
                            *selected_device_config_option = None;
                        }
                        AppEvent::ChooseDeviceButton => {
                            self.switch_document(
                                Box::new(DeviceConfigurator::new()),
                                wrapper.api.clone(),
                                wrapper.document_id,
                                wrapper.global_state.clone(),
                            );

                            let device_id_vec =
                                wrapper.global_state.device_id_vec_mutex.lock_poisoned();
                            let mut selected_device_id_option = wrapper
                                .global_state
                                .selected_device_id_option_mutex
                                .lock_poisoned();

                            *selected_device_id_option =
                                Some(device_id_vec[hit_items[0].tag.1 as usize].clone());
                            wrapper.global_state.push_connection_event(
                                ConnectionEvent::RequestDeviceConfig(
                                    device_id_vec[hit_items[0].tag.1 as usize].clone(),
                                ),
                            );
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    fn update_over_states(
        &mut self,
        hit_items: Vec<HitTestItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let mut new_over_state = HashSet::new();

        for hit_item in hit_items {
            if let Some(event) = AppEvent::from(hit_item.tag.0) {
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
                | AppEvent::MinimizeButton
                | AppEvent::ReturnButton = event
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

    fn calculate_wheel_scroll(
        &mut self,
        delta: PhysicalPosition<f64>,
        hit_items: Vec<HitTestItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
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

    fn update_app_state(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        // switch back to device list when the device disconnect
        let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();
        let selected_device_id_option = wrapper
            .global_state
            .selected_device_id_option_mutex
            .lock_poisoned();
        let selected_device_config_option = wrapper
            .global_state
            .selected_device_config_option_mutex
            .lock_poisoned();

        let mut switch_to_device_list =
            |mut selected_device_id_option: MutexGuard<Option<DeviceId>>,
             mut selected_device_config_option: MutexGuard<Option<DeviceConfig>>| {
                self.switch_document(
                    Box::new(DeviceList::new()),
                    wrapper.api.clone(),
                    wrapper.document_id,
                    wrapper.global_state.clone(),
                );

                *selected_device_id_option = None;
                *selected_device_config_option = None;
            };

        if let Some(selected_device_id) = selected_device_id_option.clone() {
            if let Some(driver) = driver_hashmap.get(&selected_device_id.thread_id) {
                if !driver
                    .device_list
                    .serial_number_vec
                    .contains(&selected_device_id.serial_number)
                {
                    switch_to_device_list(selected_device_id_option, selected_device_config_option);
                }
            } else {
                switch_to_device_list(selected_device_id_option, selected_device_config_option);
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
        let mut font_hashmap = HashMap::new();

        font_hashmap.insert(
            "OpenSans_15px",
            wrapper.load_font("OpenSans", Au::from_f32_px(15.0)),
        );
        font_hashmap.insert(
            "OpenSans_13px",
            wrapper.load_font("OpenSans", Au::from_f32_px(13.0)),
        );
        font_hashmap.insert(
            "OpenSans_10px",
            wrapper.load_font("OpenSans", Au::from_f32_px(10.0)),
        );

        Box::new(Self {
            font_hashmap,
            do_exit: false,
            over_states: HashSet::new(),
            close_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            maximize_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            minimize_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
            return_button_color_key: wrapper.api.borrow().generate_property_binding_key(),
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
            return_button_color_animation: Animation::new(
                ColorF::new_u(33, 33, 33, 100),
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
    fn on_event(
        &mut self,
        event: Event,
        hit_items: Vec<HitTestItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        match event {
            Event::Resized => {
                self.scroll_frame_size = LayoutSize::new(
                    wrapper.window_size.width as f32 - 20.0,
                    wrapper.window_size.height as f32 - 65.0,
                );

                self.update_over_states(hit_items, wrapper);
            }
            Event::MouseEntered | Event::MouseLeft => {
                self.update_over_states(hit_items, wrapper);
            }
            Event::MousePressed(MouseButton::Left) => {
                self.calculate_event(hit_items, wrapper, AppEventType::MousePressed);
            }
            Event::MouseReleased(MouseButton::Left) => {
                self.calculate_event(hit_items, wrapper, AppEventType::MouseReleased);
            }
            Event::MousePosition => {
                self.update_over_states(hit_items, wrapper);
            }
            Event::MouseWheel(delta) => {
                self.calculate_wheel_scroll(delta, hit_items.clone(), wrapper);
                self.update_over_states(hit_items, wrapper);
            }
            Event::DeviceMotion(delta) => {
                self.update_window_resize(delta, wrapper);
            }
            Event::DeviceReleased(button) => {
                // mouse left button
                if button == 1 {
                    self.resizing = None;

                    self.update_over_states(hit_items, wrapper);
                }
            }
            _ => {}
        }
    }

    fn should_exit(&self) -> bool {
        self.do_exit
    }

    fn animate(&mut self, txn: &mut Transaction, wrapper: &mut WindowWrapper<GlobalState>) {
        self.update_app_state(wrapper);
        self.animate_title_bar(txn);
        self.document.animate(txn, wrapper);
    }

    fn redraw(
        &mut self,
        frame_builder: &mut FrameBuilder,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        frame_builder.builder.push_simple_stacking_context(
            frame_builder.bounds.min,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
        );

        let background_size = LayoutRect::from_size(LayoutSize::new(
            wrapper.window_size.width as f32,
            wrapper.window_size.height as f32,
        ));

        frame_builder.builder.push_rect(
            &CommonItemProperties::new(background_size, frame_builder.space_and_clip),
            background_size,
            ColorF::new_u(33, 33, 33, 240),
        );

        // calcultate the scroll frame content size
        self.scroll_content_size = self
            .document
            .calculate_size(self.scroll_frame_size, wrapper);

        // scroll frame / main frame
        frame_builder.builder.push_simple_stacking_context(
            LayoutPoint::new(10.0, 55.0),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
        );

        let clip_chain_id = frame_builder
            .builder
            .define_clip_chain(None, [frame_builder.space_and_clip.clip_id]);

        frame_builder.builder.push_hit_test(
            LayoutRect::from_size(self.scroll_frame_size),
            clip_chain_id,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
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
            clip_chain_id,
            &self.font_hashmap,
            wrapper,
        );

        frame_builder.builder.pop_stacking_context();

        // draw main window elements
        self.draw_title_bar(
            self.document.get_title(),
            wrapper.window_size,
            frame_builder,
            clip_chain_id,
            wrapper.global_state.clone(),
        );
        self.draw_window_resize(wrapper.window_size, frame_builder);

        frame_builder.builder.pop_stacking_context();
    }

    fn set_scroll_offsets(&mut self, txn: &mut Transaction) {
        self.scroll_offset = LayoutVector2D::new(
            self.scroll_offset
                .x
                .min((self.scroll_content_size.width - self.scroll_frame_size.width).max(0.0)),
            self.scroll_offset
                .y
                .min((self.scroll_content_size.height - self.scroll_frame_size.height).max(0.0)),
        );

        txn.set_scroll_offsets(
            ExternalScrollId(EXT_SCROLL_ID_ROOT, PipelineId::dummy()),
            vec![SampledScrollOffset {
                offset: self.scroll_offset,
                generation: APZScrollGeneration::default(),
            }],
        );
    }

    fn unload(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        for font in self.font_hashmap.values_mut() {
            font.unload();
        }

        self.document
            .unload(wrapper.api.clone(), wrapper.document_id);
    }
}

pub trait DocumentTrait {
    fn get_title(&self) -> &'static str;

    fn animate(&mut self, txn: &mut Transaction, wrapper: &mut WindowWrapper<GlobalState>);

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
        clip_chain_id: ClipChainId,
        font_hashmap: &HashMap<&'static str, Font>,
        wrapper: &mut WindowWrapper<GlobalState>,
    );

    fn unload(&mut self, api: Rc<RefCell<RenderApi>>, document_id: DocumentId);
}
