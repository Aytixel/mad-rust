use webrender::api::units::LayoutSize;
use webrender::api::{
    BorderRadius, ClipId, ClipMode, ColorF, CommonItemProperties, ComplexClipRegion,
    DisplayListBuilder, PropertyBinding, SpaceAndClipInfo,
};

pub trait ColorFTrait {
    fn new_u(r: u8, g: u8, b: u8, a: u8) -> ColorF;
}

impl ColorFTrait for ColorF {
    fn new_u(r: u8, g: u8, b: u8, a: u8) -> ColorF {
        ColorF::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }
}
pub trait CommonItemPropertiesExt {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo;
}

impl CommonItemPropertiesExt for CommonItemProperties {
    fn to_space_and_clip_info(&self) -> SpaceAndClipInfo {
        SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_chain_id: self.clip_chain_id,
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
            common.spatial_id,
            ComplexClipRegion::new(common.clip_rect, radii, mode),
        );

        let mut common = *common;

        common.clip_chain_id = self.define_clip_chain(None, [clip_id]);

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
            common.spatial_id,
            ComplexClipRegion::new(common.clip_rect, radii, mode),
        );

        let mut common = *common;

        common.clip_chain_id = self.define_clip_chain(None, [clip_id]);

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
