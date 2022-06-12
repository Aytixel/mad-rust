use std::collections::HashMap;
use std::thread::ThreadId;
use std::time::Duration;

use crate::animation::{Animation, AnimationCurve};
use crate::ui::DocumentTrait;
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::{Font, FrameBuilder, GlobalStateTrait, WindowWrapper};
use crate::GlobalState;

use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, DynamicProperties, FilterOp,
    PrimitiveFlags, PropertyBinding, PropertyBindingKey, PropertyValue, SpaceAndClipInfo,
};
use webrender::Transaction;

use super::AppEvent;

pub struct DeviceList {
    driver_device_data_hashmap: HashMap<
        ThreadId,
        (
            bool,
            String,
            HashMap<String, (bool, Animation<f32>, PropertyBindingKey<f32>)>,
        ),
    >,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            driver_device_data_hashmap: HashMap::new(),
        }
    }
}

impl DocumentTrait for DeviceList {
    fn get_title(&self) -> &'static str {
        "Device List"
    }

    fn animate(&mut self, txn: &mut Transaction, wrapper: &mut WindowWrapper<GlobalState>) {
        let mut floats = vec![];

        for thread_id in self.driver_device_data_hashmap.clone().keys() {
            let (to_remove, _, device_data_hashmap) =
                self.driver_device_data_hashmap.get_mut(thread_id).unwrap();
            let mut has_update = false;

            for serial_number in device_data_hashmap.clone().keys() {
                let (to_remove, animation, key) =
                    device_data_hashmap.get_mut(serial_number).unwrap();

                if animation.update() {
                    floats.push(PropertyValue {
                        key: *key,
                        value: animation.value,
                    });

                    has_update = true;
                } else if *to_remove {
                    // remove the device
                    device_data_hashmap.remove(serial_number);
                    wrapper.global_state.request_redraw();
                }
            }

            if !has_update && *to_remove {
                // remove the driver
                self.driver_device_data_hashmap.remove(thread_id);
            }
        }

        if !floats.is_empty() {
            txn.append_dynamic_properties(DynamicProperties {
                transforms: vec![],
                floats,
                colors: vec![],
            });
        }
    }

    fn calculate_size(
        &mut self,
        _frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let driver_hashmap = match wrapper.global_state.driver_hashmap_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // mark to remove unused data
        for thread_id in self.driver_device_data_hashmap.clone().keys() {
            if driver_hashmap.contains_key(&thread_id) {
                // only mark to remove devices
                let (_, _, device_data_hashmap) =
                    self.driver_device_data_hashmap.get_mut(&thread_id).unwrap();

                for serial_number in device_data_hashmap.clone().keys() {
                    if !driver_hashmap[&thread_id]
                        .device_list
                        .serial_number_vec
                        .contains(serial_number)
                    {
                        let (to_remove, animation, _) =
                            device_data_hashmap.get_mut(serial_number).unwrap();

                        *to_remove = true;
                        animation.to(0.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                    }
                }
            } else {
                // mark to remove the entire driver
                let (to_remove, _, device_data_hashmap) =
                    self.driver_device_data_hashmap.get_mut(&thread_id).unwrap();

                *to_remove = true;

                for (to_remove, animation, _) in device_data_hashmap.values_mut() {
                    *to_remove = true;
                    animation.to(0.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                }
            }
        }

        let mut max_device_count = 0;

        // add new data
        for (thread_id, driver) in driver_hashmap.iter() {
            let (_, _, device_data_hashmap) = self
                .driver_device_data_hashmap
                .entry(*thread_id)
                .or_insert((
                    false,
                    driver.device_configuration_descriptor.device_name.clone(),
                    HashMap::new(),
                ));

            if driver.device_list.serial_number_vec.len() > max_device_count {
                max_device_count = driver.device_list.serial_number_vec.len();
            }

            for serial_number in driver.device_list.serial_number_vec.clone() {
                if let Some((to_remove, animation, _)) = device_data_hashmap.get_mut(&serial_number)
                {
                    // restore the old animation in case of reconnecting
                    *to_remove = false;
                    animation.to(1.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                } else {
                    // create a new animation
                    let mut animation =
                        Animation::new(0.0, |from: &f32, to: &f32, value: &mut f32, coef: f64| {
                            *value = (to - from) * coef as f32 + from
                        });

                    animation.to(1.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                    device_data_hashmap.insert(
                        serial_number,
                        (
                            false,
                            animation,
                            wrapper.api.borrow().generate_property_binding_key(),
                        ),
                    );
                }
            }
        }

        LayoutSize::new(
            max_device_count as f32 * 160.0 - 10.0,
            driver_hashmap.iter().count() as f32 * 160.0 - 10.0,
        )
    }

    fn draw(
        &self,
        _frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        font_hashmap: &HashMap<&'static str, Font>,
        _wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let builder = &mut frame_builder.builder;

        for (driver_index, (_, device_name, device_data_hashmap)) in
            self.driver_device_data_hashmap.values().enumerate()
        {
            for (device_index, (serial_number, (_, animation, key))) in
                device_data_hashmap.iter().enumerate()
            {
                let device_button_layout_point =
                    LayoutPoint::new(device_index as f32 * 160.0, driver_index as f32 * 160.0);
                let device_button_layout_rect = LayoutRect::from_origin_and_size(
                    device_button_layout_point,
                    LayoutSize::new(150.0, 150.0),
                );
                let device_button_common_item_properties =
                    &CommonItemProperties::new(device_button_layout_rect, space_and_clip);

                builder.push_simple_stacking_context_with_filters(
                    LayoutPoint::zero(),
                    space_and_clip.spatial_id,
                    PrimitiveFlags::IS_BACKFACE_VISIBLE,
                    &[FilterOp::Opacity(
                        PropertyBinding::Binding(*key, animation.value),
                        animation.value,
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
                builder.push_hit_test(
                    device_button_common_item_properties,
                    (AppEvent::CloseButton.into(), 0),
                );
                font_hashmap["OpenSans_13px"].push_text(
                    builder,
                    device_name
                        .get(0..device_name.len().min(16))
                        .unwrap_or_default()
                        .to_string(),
                    ColorF::new_u(255, 255, 255, 200),
                    device_button_layout_point + LayoutSize::new(7.5, 7.5),
                    space_and_clip,
                    None,
                );
                font_hashmap["OpenSans_10px"].push_text(
                    builder,
                    serial_number
                        .get(0..serial_number.len().min(21))
                        .unwrap_or_default()
                        .to_string(),
                    ColorF::new_u(255, 255, 255, 100),
                    device_button_layout_point + LayoutSize::new(7.5, 130.0),
                    space_and_clip,
                    None,
                );
                builder.pop_stacking_context();
            }
        }
    }
}
