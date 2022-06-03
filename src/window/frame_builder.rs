use webrender::api::units::{DeviceIntSize, LayoutRect, LayoutSize};
use webrender::api::{DisplayListBuilder, SpaceAndClipInfo};
use webrender::euclid::Scale;

use super::WindowWrapper;

pub struct FrameBuilder {
    pub layout_size: LayoutSize,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    pub fn new(window: &mut WindowWrapper) -> Self {
        let window_size = window.get_window_size();

        window.device_size =
            DeviceIntSize::new(window_size.width as i32, window_size.height as i32);

        let layout_size =
            window.device_size.to_f32() / Scale::new(window.context.window().scale_factor() as f32);
        let mut builder = DisplayListBuilder::new(window.pipeline_id);
        let space_and_clip = SpaceAndClipInfo::root_scroll(window.pipeline_id);
        let bounds = LayoutRect::from_size(layout_size);

        builder.begin();

        Self {
            layout_size,
            builder,
            space_and_clip,
            bounds,
        }
    }
}
