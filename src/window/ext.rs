use webrender::api::units::LayoutSize;
use webrender::api::{
    BorderRadius, ClipMode, ColorF, CommonItemProperties, ComplexClipRegion, DisplayListBuilder,
    ItemTag, SpaceAndClipInfo,
};

pub trait CommonItemPropertiesExt {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo;

    fn add_item_tag(&mut self, item_tag: ItemTag) -> Self;
}

impl CommonItemPropertiesExt for CommonItemProperties {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo {
        SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: self.clip_id,
        }
    }

    fn add_item_tag(&mut self, item_tag: ItemTag) -> Self {
        self.hit_info = Some(item_tag);
        *self
    }
}

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
            &common.to_space_and_clip_info(),
            common.clip_rect,
            [ComplexClipRegion::new(common.clip_rect, radii, mode)],
            None,
        );

        let mut common = *common;

        common.clip_id = clip_id;

        self.push_rect(&common, color);
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
