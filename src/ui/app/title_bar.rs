use std::sync::Arc;
use std::time::Duration;

use crate::animation::AnimationCurve;
use crate::ui::{App, AppEvent};
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::FrameBuilder;
use crate::GlobalState;

use hashbrown::HashSet;
use util::thread::MutexTrait;
use webrender::api::units::{
    LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, LayoutTransform,
};
use webrender::api::{
    BorderDetails, BorderRadius, BorderSide, BorderStyle, ClipChainId, ClipMode, ColorF,
    CommonItemProperties, DynamicProperties, NormalBorder, PrimitiveFlags, PropertyBinding,
    PropertyValue, ReferenceFrameKind, SpaceAndClipInfo, SpatialTreeItemKey, TransformStyle,
};
use webrender::euclid::Angle;
use webrender::Transaction;
use winit::dpi::PhysicalSize;

impl App {
    pub fn update_title_bar_over_state(&mut self, new_over_state: &HashSet<AppEvent>) {
        if new_over_state.contains(&AppEvent::CloseButton) {
            self.close_button_color_animation.to(
                ColorF::new_u(255, 79, 0, 150),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.close_button_color_animation.to(
                ColorF::new_u(255, 79, 0, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
        if new_over_state.contains(&AppEvent::MaximizeButton) {
            self.maximize_button_color_animation.to(
                ColorF::new_u(255, 189, 0, 150),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.maximize_button_color_animation.to(
                ColorF::new_u(255, 189, 0, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
        if new_over_state.contains(&AppEvent::MinimizeButton) {
            self.minimize_button_color_animation.to(
                ColorF::new_u(50, 221, 23, 150),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.minimize_button_color_animation.to(
                ColorF::new_u(50, 221, 23, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
        if new_over_state.contains(&AppEvent::ReturnButton) {
            self.return_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.return_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 0),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
    }

    pub fn animate_title_bar(&mut self, txn: &mut Transaction) {
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
        if self.return_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.return_button_color_key,
                value: self.return_button_color_animation.value,
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

    pub fn draw_title_bar(
        &mut self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
        clip_chain_id: ClipChainId,
        global_state: Arc<GlobalState>,
    ) {
        let builder = &mut frame_builder.builder;
        let has_previous_document = global_state
            .selected_device_id_option_mutex
            .lock_poisoned()
            .is_some();

        // title bar
        let title_bar_layout_rect = LayoutRect::from_origin_and_size(
            LayoutPoint::new(10.0, 10.0),
            LayoutSize::new(window_size.width as f32 - 20.0, 35.0),
        );
        let title_bar_common_item_properties =
            &CommonItemProperties::new(title_bar_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect(
            title_bar_common_item_properties,
            ColorF::new_u(66, 66, 66, 100),
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            title_bar_layout_rect,
            clip_chain_id,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
            (AppEvent::TitleBar.into(), 0),
        );

        // return button
        if has_previous_document {
            let return_button_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::new(20.0, 15.0),
                LayoutSize::new(35.0, 25.0),
            );
            let return_button_common_item_properties =
                &CommonItemProperties::new(return_button_layout_rect, frame_builder.space_and_clip);

            builder.push_rounded_rect_with_animation(
                &return_button_common_item_properties,
                PropertyBinding::Binding(
                    self.return_button_color_key,
                    self.return_button_color_animation.value,
                ),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );
            builder.push_hit_test(
                return_button_layout_rect,
                clip_chain_id,
                frame_builder.space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                (AppEvent::ReturnButton.into(), 0),
            );

            // arrow
            let spatial_id = builder.push_reference_frame(
                LayoutPoint::new(32.0, 27.5),
                frame_builder.space_and_clip.spatial_id,
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
                SpatialTreeItemKey::new(1, 0),
            );
            let return_border_layout_rect = LayoutRect::from_size(LayoutSize::splat(10.0));
            let return_border_common_item_properties = &CommonItemProperties::new(
                return_border_layout_rect,
                SpaceAndClipInfo {
                    spatial_id,
                    clip_chain_id: frame_builder.space_and_clip.clip_chain_id,
                },
            );
            let white_border_side = BorderSide {
                color: ColorF::WHITE,
                style: BorderStyle::Solid,
            };
            let transparent_border_side = BorderSide {
                color: ColorF::TRANSPARENT,
                style: BorderStyle::Solid,
            };

            builder.push_border(
                return_border_common_item_properties,
                return_border_layout_rect,
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
            builder.pop_reference_frame();
        }

        // title
        self.title_text.push_text(
            builder,
            frame_builder.space_and_clip,
            LayoutPoint::new(if has_previous_document { 65.0 } else { 20.0 }, 17.0), // if has a previous document let place for the return button
            ColorF::WHITE,
            None,
        );

        // close button
        let close_button_layout_rect = LayoutRect::from_origin_and_size(
            LayoutPoint::new(window_size.width as f32 - 55.0, 15.0),
            LayoutSize::new(35.0, 25.0),
        );
        let close_button_common_item_properties =
            &CommonItemProperties::new(close_button_layout_rect, frame_builder.space_and_clip);

        builder.push_rounded_rect_with_animation(
            &close_button_common_item_properties,
            PropertyBinding::Binding(
                self.close_button_color_key,
                self.close_button_color_animation.value,
            ),
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            close_button_layout_rect,
            clip_chain_id,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
            (AppEvent::CloseButton.into(), 0),
        );

        // maximize button
        let maximize_button_layout_rect = LayoutRect::from_origin_and_size(
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
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            maximize_button_layout_rect,
            clip_chain_id,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
            (AppEvent::MaximizeButton.into(), 0),
        );

        // minimize button
        let minimize_button_layout_rect = LayoutRect::from_origin_and_size(
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
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            minimize_button_layout_rect,
            clip_chain_id,
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::empty(),
            (AppEvent::MinimizeButton.into(), 0),
        );
    }
}
