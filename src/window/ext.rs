use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipId, ClipMode, ColorF, CommonItemProperties, ComplexClipRegion,
    DisplayListBuilder, PropertyBinding, SpaceAndClipInfo,
};

pub trait CommonItemPropertiesExt {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo;
}

impl CommonItemPropertiesExt for CommonItemProperties {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo {
        SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: self.clip_id,
        }
    }
}

pub trait DisplayListBuilderExt {
    fn push_rounded_rect(
        &mut self,
        common: &CommonItemProperties,
        color: ColorF,
        radii: BorderRadius,
        mode: ClipMode,
    ) -> ClipId;

    fn push_rounded_rect_with_animation(
        &mut self,
        common: &CommonItemProperties,
        color: PropertyBinding<ColorF>,
        radii: BorderRadius,
        mode: ClipMode,
    ) -> ClipId;
}

impl DisplayListBuilderExt for DisplayListBuilder {
    fn push_rounded_rect(
        &mut self,
        common: &CommonItemProperties,
        color: ColorF,
        radii: BorderRadius,
        mode: ClipMode,
    ) -> ClipId {
        let clip_id = self.define_clip_rounded_rect(
            &common.to_space_and_clip_info(),
            ComplexClipRegion::new(common.clip_rect, radii, mode),
        );

        let mut common = *common;

        common.clip_id = clip_id;

        self.push_rect(&common, common.clip_rect, color);

        clip_id
    }

    fn push_rounded_rect_with_animation(
        &mut self,
        common: &CommonItemProperties,
        color: PropertyBinding<ColorF>,
        radii: BorderRadius,
        mode: ClipMode,
    ) -> ClipId {
        let clip_id = self.define_clip_rounded_rect(
            &common.to_space_and_clip_info(),
            ComplexClipRegion::new(common.clip_rect, radii, mode),
        );

        let mut common = *common;

        common.clip_id = clip_id;

        self.push_rect_with_animation(&common, common.clip_rect, color);

        clip_id
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

pub trait LayoutRectExt {
    fn new_with_size(position: LayoutPoint, size: LayoutSize) -> LayoutRect;
}

impl LayoutRectExt for LayoutRect {
    fn new_with_size(position: LayoutPoint, size: LayoutSize) -> LayoutRect {
        LayoutRect::new(position, position + size)
    }
}
