use webrender::api::units::{DeviceIntSize, LayoutRect, LayoutSize};
use webrender::api::{DisplayListBuilder, SpaceAndClipInfo};

pub struct FrameBuilder {
    pub device_size: DeviceIntSize,
    pub layout_size: LayoutSize,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    pub fn new(
        device_size: DeviceIntSize,
        layout_size: LayoutSize,
        builder: DisplayListBuilder,
        space_and_clip: SpaceAndClipInfo,
        bounds: LayoutRect,
    ) -> Self {
        Self {
            device_size,
            layout_size,
            builder,
            space_and_clip,
            bounds,
        }
    }
}
