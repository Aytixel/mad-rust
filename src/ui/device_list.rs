use crate::ui::DocumentTrait;
use crate::window::{FrameBuilder, WindowWrapper};
use crate::GlobalState;

use webrender::api::units::{LayoutRect, LayoutSize};
use webrender::api::{ColorF, CommonItemProperties, SpaceAndClipInfo};
use webrender::Transaction;

pub struct DeviceList {}

impl DeviceList {
    pub fn new() -> Self {
        Self {}
    }
}

impl DocumentTrait for DeviceList {
    fn get_title(&self) -> &'static str {
        "Device List"
    }

    fn animate(&mut self, _txn: &mut Transaction) {}

    fn calculate_size(
        &self,
        frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
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

        let driver_hashmap = match wrapper.global_state.driver_hashmap_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
}
