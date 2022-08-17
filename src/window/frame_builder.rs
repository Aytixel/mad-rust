use webrender::api::units::{DeviceIntSize, LayoutRect, LayoutSize};
use webrender::api::{DisplayListBuilder, SpaceAndClipInfo};
use webrender::euclid::Scale;

use super::{GlobalStateTrait, WindowWrapper};

pub struct FrameBuilder {
    pub layout_size: LayoutSize,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    pub fn new<T: GlobalStateTrait>(wrapper: &mut WindowWrapper<T>) -> Self {
        let layout_size = DeviceIntSize::new(
            wrapper.window_size.width as i32,
            wrapper.window_size.height as i32,
        )
        .to_f32()
            / Scale::new(wrapper.context.window().scale_factor() as f32);
        let mut builder = DisplayListBuilder::new(wrapper.pipeline_id);

        builder.begin();

        let space_and_clip = SpaceAndClipInfo {
            spatial_id: SpaceAndClipInfo::root_scroll(wrapper.pipeline_id).spatial_id,
            clip_chain_id: builder.define_clip_chain(None, []),
        };
        let bounds = LayoutRect::from_size(layout_size);

        Self {
            layout_size,
            builder,
            space_and_clip,
            bounds,
        }
    }
}
