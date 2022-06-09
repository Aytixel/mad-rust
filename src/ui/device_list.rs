use std::collections::HashMap;
use std::thread::ThreadId;
use std::time::Duration;

use crate::animation::{self, Animation, AnimationCurve};
use crate::ui::DocumentTrait;
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt, LayoutRectExt};
use crate::window::{FrameBuilder, WindowWrapper};
use crate::GlobalState;

use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, DynamicProperties, FilterOp,
    PrimitiveFlags, PropertyBinding, PropertyBindingKey, PropertyValue, SpaceAndClipInfo,
};
use webrender::Transaction;

use super::AppEvent;

pub struct DeviceList {
    driver_animation_and_key_hashmap: HashMap<
        ThreadId,
        (
            bool,
            HashMap<String, (bool, Animation<f32>, PropertyBindingKey<f32>)>,
        ),
    >,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            driver_animation_and_key_hashmap: HashMap::new(),
        }
    }
}

impl DocumentTrait for DeviceList {
    fn get_title(&self) -> &'static str {
        "Device List"
    }

    fn animate(&mut self, txn: &mut Transaction) {
        let mut floats = vec![];

        for thread_id in self.driver_animation_and_key_hashmap.clone().keys() {
            let (to_remove, animation_and_key_hashmap) = self
                .driver_animation_and_key_hashmap
                .get_mut(thread_id)
                .unwrap();
            let mut has_update = false;

            for serial_number in animation_and_key_hashmap.clone().keys() {
                let (to_remove, animation, key) =
                    animation_and_key_hashmap.get_mut(serial_number).unwrap();

                if animation.update() {
                    floats.push(PropertyValue {
                        key: *key,
                        value: animation.value,
                    });

                    has_update = true;
                } else if *to_remove {
                    // remove the device
                    animation_and_key_hashmap.remove(serial_number);
                }
            }

            if !has_update && *to_remove {
                // remove the driver
                self.driver_animation_and_key_hashmap.remove(thread_id);
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
        frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let driver_hashmap = match wrapper.global_state.driver_hashmap_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // mark to remove unused data
        for thread_id in self.driver_animation_and_key_hashmap.clone().keys() {
            if driver_hashmap.contains_key(&thread_id) {
                // only mark to remove devices
                let (_, animation_and_key_hashmap) = self
                    .driver_animation_and_key_hashmap
                    .get_mut(&thread_id)
                    .unwrap();

                for serial_number in animation_and_key_hashmap.clone().keys() {
                    if !driver_hashmap[&thread_id]
                        .device_list
                        .serial_number_vec
                        .contains(serial_number)
                    {
                        let (to_remove, animation, _) =
                            animation_and_key_hashmap.get_mut(serial_number).unwrap();

                        *to_remove = true;
                        animation.to(0.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                    }
                }
            } else {
                // mark to remove the entire driver
                let (to_remove, animation_and_key_hashmap) = self
                    .driver_animation_and_key_hashmap
                    .get_mut(&thread_id)
                    .unwrap();

                *to_remove = true;

                for (to_remove, animation, _) in animation_and_key_hashmap.values_mut() {
                    *to_remove = true;
                    animation.to(0.0, Duration::from_millis(200), AnimationCurve::EASE_IN_OUT);
                }
            }
        }

        // add new data
        for (thread_id, driver) in driver_hashmap.iter() {
            let (_, animation_and_key_hashmap) = self
                .driver_animation_and_key_hashmap
                .entry(*thread_id)
                .or_insert((false, HashMap::new()));

            for serial_number in driver.device_list.serial_number_vec.clone() {
                if let Some((to_remove, animation, _)) =
                    animation_and_key_hashmap.get_mut(&serial_number)
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
                    animation_and_key_hashmap.insert(
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

        for (_, animation_and_key_hashmap) in self.driver_animation_and_key_hashmap.values() {
            for (_, animation, key) in animation_and_key_hashmap.values() {
                let device_button_layout_rect = LayoutRect::new_with_size(
                    LayoutPoint::new(0.0, 0.0),
                    LayoutSize::new(150.0, 150.0),
                );
                let device_button_common_item_properties = &CommonItemProperties::new(
                    device_button_layout_rect,
                    frame_builder.space_and_clip,
                );

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
                builder.pop_stacking_context();
            }
        }
    }
}
