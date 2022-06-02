use webrender::api::units::{LayoutRect, LayoutSize};
use webrender::api::{DisplayListBuilder, SpaceAndClipInfo};

pub struct FrameBuilder {
    pub layout_size: LayoutSize,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    pub fn new(
        layout_size: LayoutSize,
        builder: DisplayListBuilder,
        space_and_clip: SpaceAndClipInfo,
        bounds: LayoutRect,
    ) -> Self {
        Self {
            layout_size,
            builder,
            space_and_clip,
            bounds,
        }
    }
}
