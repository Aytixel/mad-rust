use crate::ui::App;
use crate::window::ext::LayoutRectExt;
use crate::window::{FrameBuilder, WindowWrapper};

use glutin::dpi::PhysicalSize;
use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize, LayoutVector2D};
use webrender::api::{
    APZScrollGeneration, ColorF, CommonItemProperties, ExternalScrollId, HasScrollLinkedEffect,
    PipelineId, PrimitiveFlags, SpaceAndClipInfo, SpatialTreeItemKey,
};
use webrender::Transaction;

const EXT_SCROLL_ID_ROOT: u64 = 1;

impl App {
    pub fn animate_device_list(&mut self, txn: &mut Transaction) {}

    pub fn draw_device_list(
        &self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
        wrapper: &mut WindowWrapper,
    ) {
        let builder = &mut frame_builder.builder;
        let root_space_and_clip = SpaceAndClipInfo::root_scroll(wrapper.pipeline_id);
        let scrollbox = LayoutRect::new_with_size(
            LayoutPoint::new(0.0, 0.0),
            LayoutSize::new(window_size.width as f32, window_size.height as f32),
        );

        builder.push_simple_stacking_context(
            LayoutPoint::new(10.0, 10.0),
            root_space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );
        builder.push_hit_test(
            &CommonItemProperties::new(scrollbox, root_space_and_clip),
            (u64::MAX, EXT_SCROLL_ID_ROOT as u16),
        );
        // set the scrolling clip
        let space1 = builder.define_scroll_frame(
            root_space_and_clip.spatial_id,
            ExternalScrollId(EXT_SCROLL_ID_ROOT, PipelineId::dummy()),
            LayoutRect::new_with_size(LayoutPoint::new(0.0, 0.0), LayoutSize::new(1000.0, 1000.0)),
            scrollbox,
            LayoutVector2D::zero(),
            APZScrollGeneration::default(),
            HasScrollLinkedEffect::No,
            SpatialTreeItemKey::new(0, 0),
        );
        let space_and_clip1 = SpaceAndClipInfo {
            spatial_id: space1,
            clip_id: root_space_and_clip.clip_id,
        };

        // now put some content into it.
        // start with a white background
        let info = CommonItemProperties::new(
            LayoutRect::new_with_size(LayoutPoint::new(0.0, 0.0), LayoutSize::new(1000.0, 1000.0)),
            space_and_clip1,
        );
        builder.push_rect(&info, info.clip_rect, ColorF::new(1.0, 1.0, 1.0, 1.0));

        // let's make a 50x50 blue square as a visual reference
        let info = CommonItemProperties::new(
            LayoutRect::new_with_size(LayoutPoint::new(0.0, 0.0), LayoutSize::new(50.0, 50.0)),
            space_and_clip1,
        );
        builder.push_rect(&info, info.clip_rect, ColorF::new(0.0, 0.0, 1.0, 1.0));

        // and a 50x50 green square next to it with an offset clip
        // to see what that looks like
        let info = CommonItemProperties::new(
            LayoutRect::new_with_size(LayoutPoint::new(50.0, 0.0), LayoutSize::new(100.0, 50.0))
                .intersection(&LayoutRect::new_with_size(
                    LayoutPoint::new(60.0, 10.0),
                    LayoutSize::new(110.0, 60.0),
                ))
                .unwrap(),
            space_and_clip1,
        );
        builder.push_rect(&info, info.clip_rect, ColorF::new(0.0, 1.0, 0.0, 1.0));

        builder.pop_stacking_context();
    }
}
