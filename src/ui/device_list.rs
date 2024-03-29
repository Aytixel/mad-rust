use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::vec;

use crate::animation::{Animation, AnimationCurve};
use crate::ui::DocumentTrait;
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::{FrameBuilder, GlobalStateTrait, WindowWrapper};
use crate::{ConnectionEvent, DeviceId, GlobalState};

use hashbrown::{HashMap, HashSet};
use image::imageops::{resize, FilterType};
use image::load_from_memory;
use util::thread::MutexTrait;
use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    AlphaType, BorderRadius, ClipMode, ColorF, CommonItemProperties, DocumentId, DynamicProperties,
    FilterOp, HitTestResultItem, IdNamespace, ImageData, ImageDescriptor, ImageDescriptorFlags,
    ImageFormat, ImageKey, ImageRendering, PrimitiveFlags, PropertyBinding, PropertyBindingKey,
    PropertyValue, SpaceAndClipInfo,
};
use webrender::{RenderApi, Transaction};

use super::device_configurator::DeviceConfigurator;
use super::{AppEvent, AppEventType};

pub struct DeviceIcon {
    image_key: ImageKey,
    width: f32,
    height: f32,
}

impl DeviceIcon {
    pub fn new(image_key: ImageKey, width: f32, height: f32) -> Self {
        Self {
            image_key,
            width,
            height,
        }
    }
}

#[derive(Clone)]
struct DeviceData {
    to_remove: bool,
    device_id: DeviceId,
    device_name: String,
    icon_option: Option<Rc<DeviceIcon>>,
    animation: Animation<f32>,
    over_color_animation: Animation<ColorF>,
    property_key: PropertyBindingKey<f32>,
    over_color_key: PropertyBindingKey<ColorF>,
}

impl DeviceData {
    fn new(
        device_id: DeviceId,
        device_name: String,
        icon_option: Option<Rc<DeviceIcon>>,
        animation: Animation<f32>,
        over_color_animation: Animation<ColorF>,
        property_key: PropertyBindingKey<f32>,
        over_color_key: PropertyBindingKey<ColorF>,
    ) -> Self {
        Self {
            to_remove: false,
            device_id,
            device_name,
            icon_option,
            animation,
            over_color_animation,
            property_key,
            over_color_key,
        }
    }
}

pub struct DeviceList {
    device_data_vec: Vec<DeviceData>,
    device_icon_option_hashmap: HashMap<SocketAddr, Option<Rc<DeviceIcon>>>,
    image_id: u32,
    device_icon_to_keep_hashset_option: Option<HashSet<SocketAddr>>,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            device_data_vec: Vec::new(),
            device_icon_option_hashmap: HashMap::new(),
            image_id: 0,
            device_icon_to_keep_hashset_option: None,
        }
    }
}

impl DocumentTrait for DeviceList {
    fn get_title(&self) -> &'static str {
        "Device List"
    }

    fn calculate_event(
        &mut self,
        hit_items: &Vec<HitTestResultItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
        target_event_type: AppEventType,
    ) {
        if !hit_items.is_empty() {
            if let Some(event) = AppEvent::from(hit_items[0].tag.0) {
                match target_event_type {
                    AppEventType::MouseReleased => match event {
                        AppEvent::ChooseDeviceButton => {
                            {
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

                            *wrapper
                                .global_state
                                .new_document_option_mutex
                                .lock_poisoned() = Some(Box::new(DeviceConfigurator::new(wrapper)));
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn update_over_state(&mut self, new_over_state: &HashSet<(AppEvent, u16)>) {
        for (index, device_data) in self.device_data_vec.iter_mut().enumerate() {
            if new_over_state.contains(&(AppEvent::ChooseDeviceButton, index as u16)) {
                device_data.over_color_animation.to(
                    ColorF::new_u(33, 33, 33, 100),
                    Duration::from_millis(100),
                    AnimationCurve::EASE_OUT,
                );
            } else {
                device_data.over_color_animation.to(
                    ColorF::new_u(33, 33, 33, 0),
                    Duration::from_millis(100),
                    AnimationCurve::EASE_IN,
                );
            }
        }
    }

    fn update_app_state(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        let drained_device_data_vec: Vec<DeviceData> = self.device_data_vec.drain(..).collect();
        let mut device_icon_to_keep_hashset = HashSet::new();

        for mut device_data in drained_device_data_vec {
            if device_data.animation.update() || !device_data.to_remove {
                device_icon_to_keep_hashset.insert(device_data.device_id.socket_addr);

                // keep the device if animation not ended or not to remove
                self.device_data_vec.push(device_data);
            } else {
                wrapper.global_state.request_redraw();
            }
        }

        self.device_icon_to_keep_hashset_option = Some(device_icon_to_keep_hashset);
    }

    fn animate(&mut self, txn: &mut Transaction, wrapper: &mut WindowWrapper<GlobalState>) {
        let mut floats = vec![];
        let mut colors = vec![];

        for device_data in self.device_data_vec.iter_mut() {
            if device_data.animation.update() {
                floats.push(PropertyValue {
                    key: device_data.property_key,
                    value: device_data.animation.value,
                });
            }
            if device_data.over_color_animation.update() {
                colors.push(PropertyValue {
                    key: device_data.over_color_key,
                    value: device_data.over_color_animation.value,
                });
            }
        }

        if !floats.is_empty() || !colors.is_empty() {
            txn.append_dynamic_properties(DynamicProperties {
                transforms: vec![],
                floats,
                colors,
            });
        }
        if let Some(device_icon_to_keep_hashset) = self.device_icon_to_keep_hashset_option.take() {
            let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();

            // remove unused icon
            for socket_addr in self.device_icon_option_hashmap.clone().keys() {
                if !device_icon_to_keep_hashset.contains(socket_addr)
                    && !driver_hashmap.contains_key(socket_addr)
                {
                    if let Some(device_icon) = self.device_icon_option_hashmap[socket_addr].clone()
                    {
                        txn.delete_image(device_icon.image_key);
                    }

                    self.device_icon_option_hashmap.remove(socket_addr);
                }
            }
        }
    }

    fn calculate_size(
        &mut self,
        mut frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();
        let mut device_button_layout_point = LayoutPoint::zero();
        let mut device_data_to_keep_hashset = HashSet::new();

        for (socket_addr, driver) in driver_hashmap.iter() {
            // initialize icon if needed
            if let None = self.device_icon_option_hashmap.get(socket_addr) {
                self.device_icon_option_hashmap.insert(
                    *socket_addr,
                    match load_from_memory(
                        driver
                            .driver_configuration_descriptor
                            .device_icon
                            .as_slice(),
                    ) {
                        Ok(image) => {
                            let mut height = 150.0f32;
                            let mut width = 150.0f32;

                            if image.height() > image.width() {
                                width /= image.height() as f32;
                                width *= image.width() as f32;
                            } else {
                                height /= image.width() as f32;
                                height *= image.height() as f32;
                            }

                            let image =
                                resize(&image, width as u32, height as u32, FilterType::Lanczos3);
                            let image_descriptor = ImageDescriptor::new(
                                width as i32,
                                height as i32,
                                ImageFormat::RGBA8,
                                ImageDescriptorFlags::empty(),
                            );
                            let image_data = ImageData::new(image.into_raw());
                            let image_key = ImageKey::new(IdNamespace(0), self.image_id);
                            let mut txn = Transaction::new();

                            self.image_id += 1;

                            txn.add_image(image_key, image_descriptor, image_data, None);
                            wrapper
                                .api_mutex
                                .lock_poisoned()
                                .send_transaction(wrapper.document_id, txn);

                            Some(Rc::new(DeviceIcon::new(image_key, width, height)))
                        }
                        Err(_) => None,
                    },
                );
            }

            for serial_number in driver.device_list.serial_number_vec.iter() {
                if let Some((index, _)) =
                    self.device_data_vec
                        .iter()
                        .enumerate()
                        .find(|(_, device_data)| -> bool {
                            device_data.device_id
                                == DeviceId::new(*socket_addr, serial_number.clone())
                        })
                {
                    device_data_to_keep_hashset.insert(index);
                } else {
                    // create a new device data
                    let mut animation =
                        Animation::new(0.0, |from: &f32, to: &f32, value: &mut f32, coef: f64| {
                            *value = (to - from) * coef as f32 + from
                        });

                    animation.to(1.0, Duration::from_millis(400), AnimationCurve::EASE_IN_OUT);
                    device_data_to_keep_hashset.insert(self.device_data_vec.len());

                    let (property_key, over_color_key) = {
                        let api = wrapper.api_mutex.lock_poisoned();

                        (
                            api.generate_property_binding_key(),
                            api.generate_property_binding_key(),
                        )
                    };

                    self.device_data_vec.push(DeviceData::new(
                        DeviceId::new(*socket_addr, serial_number.clone()),
                        driver.driver_configuration_descriptor.device_name.clone(),
                        self.device_icon_option_hashmap[socket_addr].clone(),
                        animation,
                        Animation::new(
                            ColorF::new_u(33, 33, 33, 0),
                            |from: &ColorF, to: &ColorF, value: &mut ColorF, coef: f64| {
                                value.a = (to.a - from.a) * coef as f32 + from.a
                            },
                        ),
                        property_key,
                        over_color_key,
                    ));
                }

                // calculate the next button position
                // 310 = current button width + spacing + next button width
                if device_button_layout_point.x < frame_size.width - 310.0 {
                    device_button_layout_point.x += 160.0;
                } else {
                    device_button_layout_point.x = 0.0;
                    device_button_layout_point.y += 160.0;
                }
            }
        }

        for (index, device_data) in self.device_data_vec.iter_mut().enumerate() {
            if !device_data_to_keep_hashset.contains(&index) {
                device_data.to_remove = true;
                device_data.animation.to(
                    0.0,
                    Duration::from_millis(400),
                    AnimationCurve::EASE_IN_OUT,
                );
            }
        }

        // 150 = current button row height
        frame_size.height = device_button_layout_point.y + 150.0;
        frame_size
    }

    fn draw(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let builder = &mut frame_builder.builder;
        let mut device_button_layout_point = LayoutPoint::zero();
        let mut device_id_vec = wrapper.global_state.device_id_vec_mutex.lock_poisoned();

        device_id_vec.clear();

        for (index, device_data) in self.device_data_vec.iter().enumerate() {
            let device_button_layout_rect = LayoutRect::from_origin_and_size(
                device_button_layout_point,
                LayoutSize::new(150.0, 150.0),
            );
            let device_button_common_item_properties =
                &CommonItemProperties::new(device_button_layout_rect, space_and_clip);

            builder.push_simple_stacking_context_with_filters(
                LayoutPoint::zero(),
                space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                &[FilterOp::Opacity(
                    PropertyBinding::Binding(device_data.property_key, device_data.animation.value),
                    device_data.animation.value,
                )],
                &[],
                &[],
            );
            builder.push_rounded_rect(
                &device_button_common_item_properties,
                ColorF::new_u(66, 66, 66, 100),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );
            builder.push_rounded_rect_with_animation(
                &device_button_common_item_properties,
                PropertyBinding::Binding(
                    device_data.over_color_key,
                    device_data.over_color_animation.value,
                ),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );

            // add hit test
            builder.push_hit_test(
                device_button_layout_rect,
                space_and_clip.clip_chain_id,
                space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                (AppEvent::ChooseDeviceButton.into(), index as u16),
            );
            device_id_vec.push(device_data.device_id.clone());

            // add icon if some
            if let Some(device_icon) = device_data.icon_option.clone() {
                let device_button_image_layout_rect = LayoutRect::from_origin_and_size(
                    device_button_layout_point
                        + LayoutSize::new(
                            (150.0 - device_icon.width) / 2.0,
                            (150.0 - device_icon.height) / 2.0,
                        ),
                    LayoutSize::new(device_icon.width, device_icon.height),
                );

                builder.push_image(
                    &CommonItemProperties::new(device_button_image_layout_rect, space_and_clip),
                    device_button_image_layout_rect,
                    ImageRendering::Auto,
                    AlphaType::PremultipliedAlpha,
                    device_icon.image_key,
                    ColorF::WHITE,
                );
            }

            let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

            font_hashmap["OpenSans_13px"]
                .create_text(
                    device_data
                        .device_name
                        .get(0..device_data.device_name.len().min(16))
                        .unwrap_or_default()
                        .to_string(),
                    None,
                )
                .push_text(
                    builder,
                    space_and_clip,
                    device_button_layout_point + LayoutSize::new(7.5, 7.5),
                    ColorF::WHITE,
                    None,
                );
            font_hashmap["OpenSans_10px"]
                .create_text(
                    device_data
                        .device_id
                        .serial_number
                        .get(0..device_data.device_id.serial_number.len().min(21))
                        .unwrap_or_default()
                        .to_string(),
                    None,
                )
                .push_text(
                    builder,
                    space_and_clip,
                    device_button_layout_point + LayoutSize::new(7.5, 130.0),
                    ColorF::WHITE,
                    None,
                );
            builder.pop_stacking_context();

            // calculate the next button position
            // 310 = current button width + spacing + next button width
            if device_button_layout_point.x < frame_size.width - 310.0 {
                device_button_layout_point.x += 160.0;
            } else {
                device_button_layout_point.x = 0.0;
                device_button_layout_point.y += 160.0;
            }
        }
    }

    fn unload(&mut self, api_mutex: Arc<Mutex<RenderApi>>, document_id: DocumentId) {
        for device_icon_option in self.device_icon_option_hashmap.values() {
            // unload image
            if let Some(device_icon) = device_icon_option {
                let mut txn = Transaction::new();

                txn.delete_image(device_icon.image_key);
                api_mutex.lock_poisoned().send_transaction(document_id, txn);
            }
        }
    }
}
