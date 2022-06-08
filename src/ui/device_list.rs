use crate::ui::DocumentTrait;
use crate::window::FrameBuilder;

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

    fn calculate_size(&self, frame_size: LayoutSize) -> LayoutSize {
        frame_size
    }

    fn draw(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
    ) {
        let builder = &mut frame_builder.builder;
    }
}
