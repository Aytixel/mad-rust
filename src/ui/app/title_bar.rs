use std::collections::HashSet;
use std::time::Duration;

use crate::animation::AnimationCurve;
use crate::ui::{App, AppEvent};
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt, LayoutRectExt};
use crate::window::{FrameBuilder, WindowWrapper};

use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, DynamicProperties, PropertyBinding,
    PropertyValue,
};
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
            BorderRadius::uniform(3.0),
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
            BorderRadius::uniform(3.0),
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
            BorderRadius::uniform(3.0),
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
            BorderRadius::uniform(3.0),
            ClipMode::Clip,
        );
        builder.push_hit_test(
            minimize_button_common_item_properties,
            (AppEvent::MinimizeButton.into(), 0),
        );
    }
}
