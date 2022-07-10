use std::{cell::RefCell, rc::Rc};

use crate::window::ext::{ColorFTrait, DisplayListBuilderExt};
use crate::window::{Font, FrameBuilder, Text, WindowWrapper};
use crate::GlobalState;

use super::DocumentTrait;

use hashbrown::HashMap;
use util::thread::MutexTrait;
use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipChainId, ClipMode, ColorF, CommonItemProperties, DocumentId, SpaceAndClipInfo,
};
use webrender::{RenderApi, Transaction};

struct Mode {
    name: Text,
    is_shift_mode: bool,
    mode: u8,
}

pub struct DeviceConfigurator {
    mode_vec: Vec<Mode>,
    device_info_text: Text,
}

impl DeviceConfigurator {
    pub fn new(
        font_hashmap: &HashMap<&'static str, Font>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> Self {
        let driver_hashmap = wrapper.global_state.driver_hashmap_mutex.lock_poisoned();
        let selected_device_id_option = wrapper
            .global_state
            .selected_device_id_option_mutex
            .lock_poisoned();
        let selected_device_id = selected_device_id_option.as_ref().unwrap();

        Self {
            mode_vec: vec![],
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
        }
    }
}

impl DocumentTrait for DeviceConfigurator {
    fn get_title(&self) -> &'static str {
        "Device Configuration"
    }

    fn animate(
        &mut self,
        font_hashmap: &HashMap<&'static str, Font>,
        txn: &mut Transaction,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        // add mode to the vec
        if self.mode_vec.is_empty() {
            if let Some(device_config) = &*wrapper
                .global_state
                .selected_device_config_option_mutex
                .lock_poisoned()
            {
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
            }
        }
    }

    fn calculate_size(
        &mut self,
        frame_size: LayoutSize,
        font_hashmap: &HashMap<&'static str, Font>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        let size = self.device_info_text.size + LayoutSize::new(20.0, 10.0);

        if !self.mode_vec.is_empty() {}

        size
    }

    fn draw(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
        clip_chain_id: ClipChainId,
        font_hashmap: &HashMap<&'static str, Font>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        let builder = &mut frame_builder.builder;

        // selected device informations
        let device_info_layout_rect = LayoutRect::from_origin_and_size(
            LayoutPoint::new(0.0, 0.0),
            self.device_info_text.size + LayoutSize::new(20.0, 10.0),
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
            LayoutPoint::new(10.0, 5.0),
            ColorF::WHITE,
            None,
        );

        if !self.mode_vec.is_empty() {}
    }

    fn unload(&mut self, api: Rc<RefCell<RenderApi>>, document_id: DocumentId) {}
}
