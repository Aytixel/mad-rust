use crate::ui::App;
use crate::window::FrameBuilder;

use webrender::api::units::{LayoutRect, LayoutSize};
use webrender::api::{ColorF, CommonItemProperties, SpaceAndClipInfo};
use webrender::Transaction;

impl App {
    pub fn _animate_device_list(&mut self, _txn: &mut Transaction) {}

    pub fn calculate_device_list_size(&self, frame_size: LayoutSize) -> LayoutSize {
        frame_size
    }

    pub fn draw_device_list(
        &self,
        frame_size: LayoutSize,
        frame_builder: &mut FrameBuilder,
        space_and_clip: SpaceAndClipInfo,
    ) {
        let builder = &mut frame_builder.builder;

        builder.push_rect(
            &CommonItemProperties::new(LayoutRect::from_size(frame_size), space_and_clip),
            LayoutRect::from_size(frame_size),
            ColorF::new(1.0, 1.0, 1.0, 1.0),
        );
    }
}
