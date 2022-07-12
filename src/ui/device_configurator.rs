use std::time::Duration;

use crate::animation::{Animation, AnimationCurve};
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::{FrameBuilder, GlobalStateTrait, Text, WindowWrapper};
use crate::GlobalState;

use super::{AppEvent, AppEventType, DocumentTrait};

use hashbrown::HashSet;
use util::thread::MutexTrait;
use webrender::api::units::{
    LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, LayoutTransform,
};
use webrender::api::{
    BorderDetails, BorderRadius, BorderSide, BorderStyle, ClipChainId, ClipMode, ColorF,
    CommonItemProperties, DynamicProperties, HitTestItem, NormalBorder, PrimitiveFlags,
    PropertyBinding, PropertyBindingKey, PropertyValue, ReferenceFrameKind, SpaceAndClipInfo,
    SpatialTreeItemKey, TransformStyle,
};
use webrender::euclid::Angle;

struct Mode {
    name: Text,
    is_shift_mode: bool,
    mode: u8,
}

pub struct DeviceConfigurator {
    mode_vec: Vec<Mode>,
    current_mode: usize,
    device_info_text: Text,
    mode_selector_previous_button_color_key: PropertyBindingKey<ColorF>,
    mode_selector_next_button_color_key: PropertyBindingKey<ColorF>,
    mode_selector_previous_button_color_animation: Animation<ColorF>,
    mode_selector_next_button_color_animation: Animation<ColorF>,
}

impl DeviceConfigurator {
    pub fn new(wrapper: &mut WindowWrapper<GlobalState>) -> Self {
        let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();
        let selected_device_id_option = wrapper
            .global_state
            .selected_device_id_option_mutex
            .lock_poisoned();
        let selected_device_id = selected_device_id_option.as_ref().unwrap();
        let over_color_animation = |from: &ColorF, to: &ColorF, value: &mut ColorF, coef: f64| {
            value.a = (to.a - from.a) * coef as f32 + from.a
        };
        let (mode_selector_previous_button_color_key, mode_selector_next_button_color_key) = {
            let api = wrapper.api_mutex.lock_poisoned();

            (
                api.generate_property_binding_key(),
                api.generate_property_binding_key(),
            )
        };

        Self {
            mode_vec: vec![],
            current_mode: 0,
            device_info_text: wrapper.global_state.font_hashmap_mutex.lock_poisoned()
                ["OpenSans_13px"]
                .create_text(
                    format!(
                        "Selected device : {} | {} n°",
                        driver_hashmap[&selected_device_id.socket_addr]
                            .driver_configuration_descriptor
                            .device_name,
                        selected_device_id.serial_number
                    ),
                    None,
                ),
            mode_selector_previous_button_color_key,
            mode_selector_next_button_color_key,
            mode_selector_previous_button_color_animation: Animation::new(
                ColorF::new_u(33, 33, 33, 0),
                over_color_animation,
            ),
            mode_selector_next_button_color_animation: Animation::new(
                ColorF::new_u(33, 33, 33, 0),
                over_color_animation,
            ),
        }
    }
}

impl DocumentTrait for DeviceConfigurator {
    fn get_title(&self) -> &'static str {
        "Device Configuration"
    }

    fn calculate_event(
        &mut self,
        hit_items: &Vec<HitTestItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
        target_event_type: AppEventType,
    ) {
        if !hit_items.is_empty() {
            if let Some(event) = AppEvent::from(hit_items[0].tag.0) {
                match target_event_type {
                    AppEventType::MouseReleased => match event {
                        AppEvent::ModeSelectorPrevious => {
                            if self.current_mode == 0 {
                                self.current_mode = self.mode_vec.len() - 1;
                            } else {
                                self.current_mode -= 1;
                            }

                            wrapper.global_state.request_redraw();
                        }
                        AppEvent::ModeSelectorNext => {
                            if self.current_mode == self.mode_vec.len() - 1 {
                                self.current_mode = 0;
                            } else {
                                self.current_mode += 1;
                            }

                            wrapper.global_state.request_redraw();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn update_over_state(&mut self, new_over_state: &HashSet<AppEvent>) {
        if new_over_state.contains(&AppEvent::ModeSelectorPrevious) {
            self.mode_selector_previous_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.mode_selector_previous_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 0),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
        if new_over_state.contains(&AppEvent::ModeSelectorNext) {
            self.mode_selector_next_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.mode_selector_next_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 0),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
    }

    fn update_app_state(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        // add mode to the vec
        if self.mode_vec.is_empty() {
            if let Some(device_config) = &*wrapper
                .global_state
                .selected_device_config_option_mutex
                .lock_poisoned()
            {
                let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

                // mode
                for i in 0..device_config.config[0][0].len() {
                    self.mode_vec.push(Mode {
                        name: font_hashmap["OpenSans_13px"]
                            .create_text(format!("Mode {}", i + 1), None),
                        is_shift_mode: false,
                        mode: i as u8,
                    });
                }

                // shift mode
                for i in 0..device_config.config[0][1].len() {
                    self.mode_vec.push(Mode {
                        name: font_hashmap["OpenSans_13px"]
                            .create_text(format!("Shift mode {}", i + 1), None),
                        is_shift_mode: true,
                        mode: i as u8,
                    });
                }

                wrapper.global_state.request_redraw();
            }
        }
    }

    fn animate(
        &mut self,
        txn: &mut webrender::Transaction,
        _wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let mut colors = vec![];

        if self.mode_selector_previous_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.mode_selector_previous_button_color_key,
                value: self.mode_selector_previous_button_color_animation.value,
            });
        }
        if self.mode_selector_next_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.mode_selector_next_button_color_key,
                value: self.mode_selector_next_button_color_animation.value,
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

    fn calculate_size(
        &mut self,
        frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let mut size = LayoutSize::new(self.device_info_text.size.width + 20.0, 25.0);

        if !self.mode_vec.is_empty() {
            size += LayoutSize::new(210.0, 25.0);
        }

        size
    }

    fn draw(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        clip_chain_id: ClipChainId,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let builder = &mut frame_builder.builder;

        // selected device informations
        let device_info_layout_rect = LayoutRect::from_origin_and_size(
            LayoutPoint::new(0.0, 0.0),
            LayoutSize::new(self.device_info_text.size.width + 20.0, 25.0),
        );
        let device_info_common_item_properties =
            &CommonItemProperties::new(device_info_layout_rect, space_and_clip);

        builder.push_rounded_rect(
            &device_info_common_item_properties,
            ColorF::new_u(66, 66, 66, 100),
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        self.device_info_text.push_text(
            builder,
            space_and_clip,
            LayoutPoint::new(10.0, 4.0),
            ColorF::WHITE,
            None,
        );

        if !self.mode_vec.is_empty() {
            let current_mode = &self.mode_vec[self.current_mode];

            // mode selector
            let mode_selector_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::new(device_info_layout_rect.width() + 10.0, 0.0),
                LayoutSize::new(200.0, 25.0),
            );
            let mode_selector_common_item_properties =
                &CommonItemProperties::new(mode_selector_layout_rect, space_and_clip);

            builder.push_rounded_rect(
                &mode_selector_common_item_properties,
                ColorF::new_u(66, 66, 66, 100),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );

            // mode selector text
            current_mode.name.push_text(
                builder,
                space_and_clip,
                LayoutPoint::new(mode_selector_layout_rect.x_range().start + 35.0 + 10.0, 4.0),
                ColorF::WHITE,
                None,
            );

            // mode selector previous
            let mode_selector_previous_button_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::new(mode_selector_layout_rect.x_range().start, 0.0),
                LayoutSize::new(35.0, 25.0),
            );
            let mode_selector_previous_button_common_item_properties = &CommonItemProperties::new(
                mode_selector_previous_button_layout_rect,
                space_and_clip,
            );

            builder.push_rounded_rect_with_animation(
                &mode_selector_previous_button_common_item_properties,
                PropertyBinding::Binding(
                    self.mode_selector_previous_button_color_key,
                    self.mode_selector_previous_button_color_animation.value,
                ),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );
            builder.push_hit_test(
                mode_selector_previous_button_layout_rect,
                clip_chain_id,
                space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                (AppEvent::ModeSelectorPrevious.into(), 0),
            );

            // mode selector next
            let mode_selector_next_button_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::new(mode_selector_layout_rect.x_range().end - 35.0, 0.0),
                LayoutSize::new(35.0, 25.0),
            );
            let mode_selector_next_button_common_item_properties =
                &CommonItemProperties::new(mode_selector_next_button_layout_rect, space_and_clip);

            builder.push_rounded_rect_with_animation(
                &mode_selector_next_button_common_item_properties,
                PropertyBinding::Binding(
                    self.mode_selector_next_button_color_key,
                    self.mode_selector_next_button_color_animation.value,
                ),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );
            builder.push_hit_test(
                mode_selector_next_button_layout_rect,
                clip_chain_id,
                space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                (AppEvent::ModeSelectorNext.into(), 0),
            );

            // mode selector arrows
            let spatial_id = builder.push_reference_frame(
                LayoutPoint::new(mode_selector_layout_rect.x_range().start, 12.5),
                space_and_clip.spatial_id,
                TransformStyle::Flat,
                PropertyBinding::Value(LayoutTransform::rotation(
                    0.0,
                    0.0,
                    1.0,
                    Angle::degrees(-45.0),
                )),
                ReferenceFrameKind::Transform {
                    is_2d_scale_translation: false,
                    should_snap: false,
                    paired_with_perspective: false,
                },
                SpatialTreeItemKey::new(2, 0),
            );
            let white_border_side = BorderSide {
                color: ColorF::WHITE,
                style: BorderStyle::Solid,
            };
            let transparent_border_side = BorderSide {
                color: ColorF::TRANSPARENT,
                style: BorderStyle::Solid,
            };
            let mode_selector_left_arrow_layout_rect =
                LayoutRect::from_origin_and_size(LayoutPoint::splat(8.5), LayoutSize::splat(10.0));
            let mode_selector_left_arrow_common_item_properties = &CommonItemProperties::new(
                mode_selector_left_arrow_layout_rect,
                SpaceAndClipInfo {
                    spatial_id,
                    clip_id: space_and_clip.clip_id,
                },
            );

            builder.push_border(
                mode_selector_left_arrow_common_item_properties,
                mode_selector_left_arrow_layout_rect,
                LayoutSideOffsets::new_all_same(1.0),
                BorderDetails::Normal(NormalBorder {
                    left: white_border_side,
                    right: transparent_border_side,
                    top: white_border_side,
                    bottom: transparent_border_side,
                    radius: BorderRadius::zero(),
                    do_aa: false,
                }),
            );

            let mode_selector_right_arrow_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::splat(123.0),
                LayoutSize::splat(10.0),
            );
            let mode_selector_right_arrow_common_item_properties = &CommonItemProperties::new(
                mode_selector_right_arrow_layout_rect,
                SpaceAndClipInfo {
                    spatial_id,
                    clip_id: space_and_clip.clip_id,
                },
            );

            builder.push_border(
                mode_selector_right_arrow_common_item_properties,
                mode_selector_right_arrow_layout_rect,
                LayoutSideOffsets::new_all_same(1.0),
                BorderDetails::Normal(NormalBorder {
                    left: transparent_border_side,
                    right: white_border_side,
                    top: transparent_border_side,
                    bottom: white_border_side,
                    radius: BorderRadius::zero(),
                    do_aa: false,
                }),
            );
            builder.pop_reference_frame();
        }
    }
}
