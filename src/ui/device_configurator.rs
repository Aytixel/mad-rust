use hashbrown::HashMap;
use webrender::{
    api::{units::LayoutSize, ClipChainId, SpaceAndClipInfo},
    Transaction,
};

use crate::{
    window::{Font, FrameBuilder, WindowWrapper},
    GlobalState,
};

use super::DocumentTrait;

pub struct DeviceConfigurator {}

impl DeviceConfigurator {
    pub fn new() -> Self {
        Self {}
    }
}

impl DocumentTrait for DeviceConfigurator {
    fn get_title(&self) -> &'static str {
        "Device Configuration"
    }

    fn animate(&mut self, txn: &mut Transaction, wrapper: &mut WindowWrapper<GlobalState>) {}

    fn calculate_size(
        &mut self,
        frame_size: LayoutSize,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) -> LayoutSize {
        LayoutSize::zero()
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
    }

    fn unload(&mut self, wrapper: &mut WindowWrapper<GlobalState>) {}
}
