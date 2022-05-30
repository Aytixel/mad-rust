use webrender::api::units::LayoutSize;
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, ComplexClipRegion, DisplayListBuilder,
    SpaceAndClipInfo,
};

pub trait DisplayListBuilderExt {
    fn push_rounded_rect(
        &mut self,
        common: &CommonItemProperties,
        color: ColorF,
        radii: BorderRadius,
        mode: ClipMode,
    );
}

impl DisplayListBuilderExt for DisplayListBuilder {
    fn push_rounded_rect(
        &mut self,
        common: &CommonItemProperties,
        color: ColorF,
        radii: BorderRadius,
        mode: ClipMode,
    ) {
        let clip_id = self.define_clip(
            &SpaceAndClipInfo {
                spatial_id: common.spatial_id,
                clip_id: common.clip_id,
            },
            common.clip_rect,
            [ComplexClipRegion::new(common.clip_rect, radii, mode)],
            None,
        );

        self.push_rect(
            &CommonItemProperties::new(
                common.clip_rect,
                SpaceAndClipInfo {
                    spatial_id: common.spatial_id,
                    clip_id: clip_id,
                },
            ),
            color,
        );
    }
}

pub trait BorderRadiusExt {
    fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> BorderRadius;
}

impl BorderRadiusExt for BorderRadius {
    fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> BorderRadius {
        BorderRadius {
            top_left: LayoutSize::new(top_left, top_left),
            top_right: LayoutSize::new(top_right, top_right),
            bottom_left: LayoutSize::new(bottom_left, bottom_left),
            bottom_right: LayoutSize::new(bottom_right, bottom_right),
        }
    }
}
