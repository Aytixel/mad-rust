use std::sync::Mutex;
use std::time::Duration;

use crate::animation::{Animation, AnimationCurve};
use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::{Font, FrameBuilder, GlobalStateTrait, Text, WindowWrapper};
use crate::{ConnectionEvent, GlobalState};

use super::{AppEvent, AppEventType, DocumentTrait};

use copypasta::{ClipboardContext, ClipboardProvider};
use hashbrown::HashSet;
use util::connection::command::DeviceConfig;
use util::thread::MutexTrait;
use util::time::Timer;
use webrender::api::units::{
    LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, LayoutTransform,
};
use webrender::api::{
    BorderDetails, BorderRadius, BorderSide, BorderStyle, ClipMode, ColorF, CommonItemProperties,
    DisplayListBuilder, DynamicProperties, GlyphOptions, HitTestResultItem, NormalBorder,
    PrimitiveFlags, PropertyBinding, PropertyBindingKey, PropertyValue, ReferenceFrameKind,
    SpaceAndClipInfo, SpatialTreeItemKey, TransformStyle,
};
use webrender::euclid::Angle;
use webrender::{RenderApi, Transaction};
use winit::event::VirtualKeyCode;

struct Mode {
    name: Text,
    is_shift_mode: bool,
    mode: u8,
}

struct TextInput {
    text: String,
    focused: bool,
    first_text: Text,
    second_text: Text,
    width: f32,
    height: f32,
    cursor_height: f32,
    cursor_position: usize,
    cursor_color_key: PropertyBindingKey<ColorF>,
    cursor_color: ColorF,
    cursor_color_state: bool,
    cursor_timer: Timer,
}

impl TextInput {
    fn new(
        mut text: String,
        font: &Font,
        api_mutex: &Mutex<RenderApi>,
        cursor_color: ColorF,
        cursor_height: f32,
    ) -> Self {
        text.retain(|c| c != '\n' && c != '\r');

        let first_text = font.create_text(text[..0].to_string(), None);
        let second_text = font.create_text(text[0..].to_string(), None);

        Self {
            text,
            focused: false,
            first_text,
            width: second_text.size.width,
            height: second_text.size.height,
            second_text,
            cursor_height,
            cursor_position: 0,
            cursor_color_key: api_mutex.lock_poisoned().generate_property_binding_key(),
            cursor_color,
            cursor_color_state: true,
            cursor_timer: Timer::new(Duration::from_millis(350)),
        }
    }

    fn set_focus(&mut self, focus: bool) {
        self.focused = focus;
        self.width = self.first_text.size.width
            + self.second_text.size.width
            + (self.focused as u8 as f32 * 5.0);
    }

    fn update_text(&mut self, font: &Font) {
        let (first_text, second_text) = self.text.split_at(self.cursor_position);

        self.first_text = font.create_text(first_text.to_string(), None);
        self.second_text = font.create_text(second_text.to_string(), None);
        self.width = self.first_text.size.width
            + self.second_text.size.width
            + (self.focused as u8 as f32 * 5.0);
        self.height = self
            .first_text
            .size
            .height
            .max(self.second_text.size.height);
    }

    fn add_char(&mut self, font: &Font, char: char) {
        self.text.insert(self.cursor_position, char);
        self.cursor_position += 1;

        while !self.text.is_char_boundary(self.cursor_position) {
            self.cursor_position += 1;
        }

        self.update_text(font);
    }

    fn add_str(&mut self, font: &Font, text: &str) {
        self.text.insert_str(self.cursor_position, text);
        self.cursor_position += text.len();
        self.update_text(font);
    }

    fn delete_char(&mut self, font: &Font) {
        if self.text.len() > self.cursor_position {
            self.text.remove(self.cursor_position);
        }

        self.update_text(font);
    }

    fn back_char(&mut self, font: &Font) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;

            while !self.text.is_char_boundary(self.cursor_position) {
                self.cursor_position -= 1;
            }

            self.text.remove(self.cursor_position);
        }

        self.update_text(font);
    }

    fn change_cursor_position(&mut self, font: &Font, cursor_position: usize) {
        self.cursor_position = cursor_position.min(self.text.len());

        while !self.text.is_char_boundary(self.cursor_position) {
            self.cursor_position += 1;
        }

        self.update_text(font);
    }

    fn cursor_left(&mut self, font: &Font) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;

            while !self.text.is_char_boundary(self.cursor_position) {
                self.cursor_position -= 1;
            }

            self.change_cursor_position(font, self.cursor_position);
        }
    }

    fn cursor_right(&mut self, font: &Font) {
        if self.cursor_position < usize::MAX {
            self.change_cursor_position(font, self.cursor_position + 1);
        }
    }

    fn animate(&mut self) -> Option<PropertyValue<ColorF>> {
        if self.cursor_timer.check() {
            self.cursor_color_state = !self.cursor_color_state;

            Some(PropertyValue {
                key: self.cursor_color_key,
                value: if self.cursor_color_state {
                    self.cursor_color
                } else {
                    ColorF::TRANSPARENT
                },
            })
        } else {
            None
        }
    }

    fn push_text(
        &self,
        builder: &mut DisplayListBuilder,
        space_and_clip: SpaceAndClipInfo,
        position: LayoutPoint,
        color: ColorF,
        glyph_options: Option<GlyphOptions>,
    ) {
        self.first_text
            .push_text(builder, space_and_clip, position, color, glyph_options);

        if self.focused {
            let cursor_layout_rect = LayoutRect::from_origin_and_size(
                position + LayoutSize::new(self.first_text.size.width + 2.0, 0.0),
                LayoutSize::new(1.0, self.cursor_height),
            );
            let cursor_common_item_properties =
                &CommonItemProperties::new(cursor_layout_rect, space_and_clip);

            builder.push_rect_with_animation(
                cursor_common_item_properties,
                cursor_layout_rect,
                PropertyBinding::Binding(self.cursor_color_key, self.cursor_color),
            );
        }

        self.second_text.push_text(
            builder,
            space_and_clip,
            position
                + LayoutSize::new(
                    self.first_text.size.width + (self.focused as u8 as f32 * 5.0),
                    0.0,
                ),
            color,
            glyph_options,
        );
    }
}

struct Parameter {
    name: Text,
    value: TextInput,
}

pub struct DeviceConfigurator {
    mode_vec: Vec<Mode>,
    parameter_vec: Vec<Parameter>,
    apply_configcurrent_focused_parameter_index_option: Option<usize>,
    current_mode: usize,
    device_info_text: Text,
    apply_config_text: Text,
    clipboard_context: ClipboardContext,
    mode_selector_previous_button_color_key: PropertyBindingKey<ColorF>,
    mode_selector_next_button_color_key: PropertyBindingKey<ColorF>,
    apply_config_button_color_key: PropertyBindingKey<ColorF>,
    mode_selector_previous_button_color_animation: Animation<ColorF>,
    mode_selector_next_button_color_animation: Animation<ColorF>,
    apply_config_button_color_animation: Animation<ColorF>,
}

impl DeviceConfigurator {
    pub fn new(wrapper: &mut WindowWrapper<GlobalState>) -> Self {
        let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();
        let selected_device_id_option = wrapper
            .global_state
            .selected_device_id_option_mutex
            .lock_poisoned();
        let selected_device_id = selected_device_id_option.as_ref().unwrap();
        let button_color_animation = Animation::new(
            ColorF::new_u(33, 33, 33, 0),
            |from: &ColorF, to: &ColorF, value: &mut ColorF, coef: f64| {
                value.a = (to.a - from.a) * coef as f32 + from.a
            },
        );
        let (
            mode_selector_previous_button_color_key,
            mode_selector_next_button_color_key,
            apply_config_button_color_key,
        ) = {
            let api = wrapper.api_mutex.lock_poisoned();

            (
                api.generate_property_binding_key(),
                api.generate_property_binding_key(),
                api.generate_property_binding_key(),
            )
        };
        let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

        Self {
            mode_vec: vec![],
            parameter_vec: vec![],
            apply_configcurrent_focused_parameter_index_option: None,
            current_mode: 0,
            device_info_text: font_hashmap["OpenSans_13px"].create_text(
                format!(
                    "Selected device : {} | {} n°",
                    driver_hashmap[&selected_device_id.socket_addr]
                        .driver_configuration_descriptor
                        .device_name,
                    selected_device_id.serial_number
                ),
                None,
            ),
            apply_config_text: font_hashmap["OpenSans_13px"]
                .create_text("Apply config".to_string(), None),
            clipboard_context: ClipboardContext::new().unwrap(),
            mode_selector_previous_button_color_key,
            mode_selector_next_button_color_key,
            apply_config_button_color_key,
            mode_selector_previous_button_color_animation: button_color_animation.clone(),
            mode_selector_next_button_color_animation: button_color_animation.clone(),
            apply_config_button_color_animation: button_color_animation,
        }
    }

    fn update_parameter(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        if let Some(selected_device_config) = wrapper
            .global_state
            .selected_device_config_option_mutex
            .lock_poisoned()
            .as_ref()
        {
            let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

            for (index, parameter) in self.parameter_vec.iter_mut().enumerate() {
                let is_shift_mode = self.mode_vec[self.current_mode].is_shift_mode;
                let mode = self.mode_vec[self.current_mode].mode;

                parameter.value = TextInput::new(
                    selected_device_config.config[index][is_shift_mode as usize][mode as usize]
                        .clone(),
                    &font_hashmap["OpenSans_13px"],
                    &wrapper.api_mutex,
                    ColorF::WHITE,
                    17.0,
                );
            }

            wrapper.global_state.request_redraw();
        }
    }

    fn update_selected_config(
        &self,
        selected_device_config_option_mutex: &Mutex<Option<DeviceConfig>>,
    ) {
        if let (Some(current_focused_parameter), Some(selected_device_config)) = (
            self.apply_configcurrent_focused_parameter_index_option,
            selected_device_config_option_mutex.lock_poisoned().as_mut(),
        ) {
            let is_shift_mode = self.mode_vec[self.current_mode].is_shift_mode;
            let mode = self.mode_vec[self.current_mode].mode;

            selected_device_config.config[current_focused_parameter][is_shift_mode as usize]
                [mode as usize] = self.parameter_vec[current_focused_parameter]
                .value
                .text
                .clone();
        }
    }
}

impl DocumentTrait for DeviceConfigurator {
    fn get_title(&self) -> &'static str {
        "Device Configuration"
    }

    fn calculate_event(
        &mut self,
        hit_items: &Vec<HitTestResultItem>,
        wrapper: &mut WindowWrapper<GlobalState>,
        target_event_type: AppEventType,
    ) {
        // parameters text input event logic
        if let Some(current_focused_parameter_index) =
            self.apply_configcurrent_focused_parameter_index_option
        {
            let current_focused_parameter =
                &mut self.parameter_vec[current_focused_parameter_index].value;

            match target_event_type {
                AppEventType::MousePressed | AppEventType::Focus(false) => {
                    for parameter in self.parameter_vec.iter_mut() {
                        parameter.value.set_focus(false);
                    }

                    self.apply_configcurrent_focused_parameter_index_option = None;

                    wrapper.global_state.request_redraw();
                }
                AppEventType::KeyPressed { keycode, modifiers } => {
                    let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

                    match keycode {
                        VirtualKeyCode::Left => {
                            current_focused_parameter.cursor_left(&font_hashmap["OpenSans_13px"]);
                            wrapper.global_state.request_redraw();
                        }
                        VirtualKeyCode::Right => {
                            current_focused_parameter.cursor_right(&font_hashmap["OpenSans_13px"]);
                            wrapper.global_state.request_redraw();
                        }
                        VirtualKeyCode::Delete => {
                            current_focused_parameter.delete_char(&font_hashmap["OpenSans_13px"]);

                            self.update_selected_config(
                                &wrapper.global_state.selected_device_config_option_mutex,
                            );

                            wrapper.global_state.request_redraw();
                        }
                        VirtualKeyCode::Back => {
                            current_focused_parameter.back_char(&font_hashmap["OpenSans_13px"]);

                            self.update_selected_config(
                                &wrapper.global_state.selected_device_config_option_mutex,
                            );

                            wrapper.global_state.request_redraw();
                        }
                        VirtualKeyCode::C | VirtualKeyCode::X => {
                            if modifiers.ctrl() {
                                self.clipboard_context
                                    .set_contents(current_focused_parameter.text.clone())
                                    .ok();
                            }
                        }
                        VirtualKeyCode::V => {
                            if modifiers.ctrl() {
                                if let Ok(mut text) = self.clipboard_context.get_contents() {
                                    text.retain(|c| c != '\n' && c != '\r');
                                    current_focused_parameter
                                        .add_str(&font_hashmap["OpenSans_13px"], text.as_str());

                                    self.update_selected_config(
                                        &wrapper.global_state.selected_device_config_option_mutex,
                                    );

                                    wrapper.global_state.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                AppEventType::Char(char) => {
                    if char != '\n'
                        && char != '\r'
                        && char != '\u{3}'
                        && char != '\u{8}'
                        && char != '\u{16}'
                        && char != '\u{18}'
                        && char != '\u{1b}'
                        && char != '\u{7f}'
                    {
                        let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

                        current_focused_parameter.add_char(&font_hashmap["OpenSans_13px"], char);

                        self.update_selected_config(
                            &wrapper.global_state.selected_device_config_option_mutex,
                        );

                        wrapper.global_state.request_redraw();
                    }
                }
                _ => {}
            }
        }

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

                            self.update_parameter(wrapper);
                        }
                        AppEvent::ModeSelectorNext => {
                            if self.current_mode == self.mode_vec.len() - 1 {
                                self.current_mode = 0;
                            } else {
                                self.current_mode += 1;
                            }

                            self.update_parameter(wrapper);
                        }
                        AppEvent::ApplyConfig => {
                            if let (Some(selected_device_id), Some(selected_device_config)) = (
                                wrapper
                                    .global_state
                                    .selected_device_id_option_mutex
                                    .lock_poisoned()
                                    .as_ref(),
                                wrapper
                                    .global_state
                                    .selected_device_config_option_mutex
                                    .lock_poisoned()
                                    .as_ref(),
                            ) {
                                wrapper.global_state.push_connection_event(
                                    ConnectionEvent::ApplyDeviceConfig(
                                        selected_device_id.socket_addr,
                                        selected_device_config.clone(),
                                    ),
                                );
                            }
                        }
                        AppEvent::Parameter => {
                            self.parameter_vec[hit_items[0].tag.1 as usize]
                                .value
                                .set_focus(true);
                            self.apply_configcurrent_focused_parameter_index_option =
                                Some(hit_items[0].tag.1 as usize);

                            wrapper.global_state.request_redraw();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn update_over_state(&mut self, new_over_state: &HashSet<(AppEvent, u16)>) {
        if new_over_state.contains(&(AppEvent::ModeSelectorPrevious, 0)) {
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
        if new_over_state.contains(&(AppEvent::ModeSelectorNext, 0)) {
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
        if new_over_state.contains(&(AppEvent::ApplyConfig, 0)) {
            self.apply_config_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 100),
                Duration::from_millis(100),
                AnimationCurve::EASE_OUT,
            );
        } else {
            self.apply_config_button_color_animation.to(
                ColorF::new_u(33, 33, 33, 0),
                Duration::from_millis(100),
                AnimationCurve::EASE_IN,
            );
        }
    }

    fn update_app_state(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {
        // add mode to the vec
        if self.mode_vec.is_empty() {
            if let (Some(selected_device_config), Some(devide_id)) = (
                wrapper
                    .global_state
                    .selected_device_config_option_mutex
                    .lock_poisoned()
                    .as_ref(),
                wrapper
                    .global_state
                    .selected_device_id_option_mutex
                    .lock_poisoned()
                    .as_ref(),
            ) {
                if let Some(driver) = wrapper
                    .global_state
                    .driver_hashmap_mutex
                    .lock_poisoned()
                    .get(&devide_id.socket_addr)
                {
                    let font_hashmap = wrapper.global_state.font_hashmap_mutex.lock_poisoned();

                    // mode
                    for i in 0..driver.driver_configuration_descriptor.mode_count {
                        self.mode_vec.push(Mode {
                            name: font_hashmap["OpenSans_13px"]
                                .create_text(format!("Mode {}", i + 1), None),
                            is_shift_mode: false,
                            mode: i as u8,
                        });
                    }

                    // shift mode
                    for i in 0..driver.driver_configuration_descriptor.shift_mode_count {
                        self.mode_vec.push(Mode {
                            name: font_hashmap["OpenSans_13px"]
                                .create_text(format!("Shift mode {}", i + 1), None),
                            is_shift_mode: true,
                            mode: i as u8,
                        });
                    }

                    // parameters
                    for (index, button_name) in driver
                        .driver_configuration_descriptor
                        .button_name_vec
                        .iter()
                        .enumerate()
                    {
                        let is_shift_mode = self.mode_vec[self.current_mode].is_shift_mode;
                        let mode = self.mode_vec[self.current_mode].mode;

                        self.parameter_vec.push(Parameter {
                            name: font_hashmap["OpenSans_13px"]
                                .create_text(format!("{button_name} : "), None),
                            value: TextInput::new(
                                selected_device_config.config[index][is_shift_mode as usize]
                                    [mode as usize]
                                    .clone(),
                                &font_hashmap["OpenSans_13px"],
                                &wrapper.api_mutex,
                                ColorF::WHITE,
                                17.0,
                            ),
                        });
                    }

                    wrapper.global_state.request_redraw();
                }
            }
        }
    }

    fn animate(&mut self, txn: &mut Transaction, _wrapper: &mut WindowWrapper<GlobalState>) {
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
        if self.apply_config_button_color_animation.update() {
            colors.push(PropertyValue {
                key: self.apply_config_button_color_key,
                value: self.apply_config_button_color_animation.value,
            });
        }

        // parameters
        for property_value in self
            .parameter_vec
            .iter_mut()
            .filter_map(|parameter| parameter.value.animate())
        {
            colors.push(property_value);
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
        _frame_size: LayoutSize,
        _wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let mut height = 25.0;
        let mut width = self.device_info_text.size.width + self.apply_config_text.size.width + 50.0;

        if !self.mode_vec.is_empty() {
            height += 25.0;
            width += 210.0;

            // parameters
            for parameter in self.parameter_vec.iter() {
                width = width.max(parameter.name.size.width + parameter.value.width + 30.0);
            }

            height += 35.0 * (self.parameter_vec.len() - 1) as f32 + 30.0;
        }

        LayoutSize::new(width, height)
    }

    fn draw(
        &self,
        _frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        _wrapper: &mut WindowWrapper<GlobalState>,
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
                space_and_clip.clip_chain_id,
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
                space_and_clip.clip_chain_id,
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
                    clip_chain_id: space_and_clip.clip_chain_id,
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
                    clip_chain_id: space_and_clip.clip_chain_id,
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

            // apply config button
            let apply_config_button_layout_rect = LayoutRect::from_origin_and_size(
                LayoutPoint::new(mode_selector_layout_rect.x_range().end + 10.0, 0.0),
                LayoutSize::new(self.apply_config_text.size.width + 20.0, 25.0),
            );
            let apply_config_button_common_item_properties =
                &CommonItemProperties::new(apply_config_button_layout_rect, space_and_clip);

            builder.push_rounded_rect(
                &apply_config_button_common_item_properties,
                ColorF::new_u(66, 66, 66, 100),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );
            builder.push_rounded_rect_with_animation(
                &apply_config_button_common_item_properties,
                PropertyBinding::Binding(
                    self.apply_config_button_color_key,
                    self.apply_config_button_color_animation.value,
                ),
                BorderRadius::uniform(3.0),
                ClipMode::Clip,
            );

            self.apply_config_text.push_text(
                builder,
                space_and_clip,
                LayoutPoint::new(mode_selector_layout_rect.x_range().end + 20.0, 4.0),
                ColorF::WHITE,
                None,
            );

            builder.push_hit_test(
                apply_config_button_layout_rect,
                space_and_clip.clip_chain_id,
                space_and_clip.spatial_id,
                PrimitiveFlags::empty(),
                (AppEvent::ApplyConfig.into(), 0),
            );

            // parameters
            let mut parameter_position = LayoutPoint::new(10.0, 45.0);

            for (index, parameter) in self.parameter_vec.iter().enumerate() {
                let parameter_layout_rect = LayoutRect::from_origin_and_size(
                    parameter_position,
                    LayoutSize::new(
                        parameter.name.size.width + parameter.value.width + 20.0,
                        25.0,
                    ),
                );
                let parameter_common_item_properties =
                    &CommonItemProperties::new(parameter_layout_rect, space_and_clip);

                builder.push_rounded_rect(
                    &parameter_common_item_properties,
                    ColorF::new_u(66, 66, 66, 100),
                    BorderRadius::uniform(3.0),
                    ClipMode::Clip,
                );
                builder.push_hit_test(
                    parameter_layout_rect,
                    space_and_clip.clip_chain_id,
                    space_and_clip.spatial_id,
                    PrimitiveFlags::empty(),
                    (AppEvent::Parameter.into(), index as u16),
                );
                parameter.name.push_text(
                    builder,
                    space_and_clip,
                    parameter_position + LayoutSize::new(10.0, 4.0),
                    ColorF::WHITE,
                    None,
                );
                parameter.value.push_text(
                    builder,
                    space_and_clip,
                    parameter_position + LayoutSize::new(parameter.name.size.width + 10.0, 4.0),
                    ColorF::WHITE,
                    None,
                );

                parameter_position += LayoutSize::new(0.0, 35.0);
            }
        }
    }
}
