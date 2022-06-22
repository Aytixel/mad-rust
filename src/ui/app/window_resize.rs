use crate::ui::{App, AppEvent};
use crate::window::{FrameBuilder, WindowWrapper};
use crate::GlobalState;

use hashbrown::HashSet;
use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    BorderRadius, ClipMode, CommonItemProperties, ComplexClipRegion, SpaceAndClipInfo,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::CursorIcon;

impl App {
    pub fn update_window_resize_cursor_icon(
        &self,
        new_over_state: &HashSet<AppEvent>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        if let None = self.resizing {
            let test_cursor = |event: &AppEvent, cursor: CursorIcon| -> bool {
                if new_over_state.contains(event) {
                    wrapper.context.window().set_cursor_icon(cursor);

                    true
                } else {
                    false
                }
            };
            let is_cursor_icon_set =
                test_cursor(&AppEvent::WindowResizeTopLeft, CursorIcon::NwResize)
                    || test_cursor(&AppEvent::WindowResizeTopRight, CursorIcon::NeResize)
                    || test_cursor(&AppEvent::WindowResizeTop, CursorIcon::NResize)
                    || test_cursor(&AppEvent::WindowResizeBottomLeft, CursorIcon::SwResize)
                    || test_cursor(&AppEvent::WindowResizeBottomRight, CursorIcon::SeResize)
                    || test_cursor(&AppEvent::WindowResizeBottom, CursorIcon::SResize)
                    || test_cursor(&AppEvent::WindowResizeLeft, CursorIcon::WResize)
                    || test_cursor(&AppEvent::WindowResizeRight, CursorIcon::EResize);

            if !is_cursor_icon_set {
                wrapper
                    .context
                    .window()
                    .set_cursor_icon(CursorIcon::Default);
            }
        }
    }

    pub fn update_window_resize(
        &self,
        delta: PhysicalPosition<f64>,
        wrapper: &mut WindowWrapper<GlobalState>,
    ) {
        if let Some(event) = self.resizing.clone() {
            let window_size = wrapper.get_window_size();
            let window_position = wrapper.get_window_position();
            let mut new_window_size =
                PhysicalSize::new(window_size.width as f64, window_size.height as f64);
            let mut new_window_position =
                PhysicalPosition::new(window_position.x as f64, window_position.y as f64);

            match event {
                AppEvent::WindowResizeTopLeft => {
                    new_window_position.x += delta.x;
                    new_window_size.width -= delta.x;
                    new_window_position.y += delta.y;
                    new_window_size.height -= delta.y;
                }
                AppEvent::WindowResizeTopRight => {
                    new_window_size.width += delta.x;
                    new_window_position.y += delta.y;
                    new_window_size.height -= delta.y;
                }
                AppEvent::WindowResizeTop => {
                    new_window_position.y += delta.y;
                    new_window_size.height -= delta.y;
                }
                AppEvent::WindowResizeBottomLeft => {
                    new_window_position.x += delta.x;
                    new_window_size.width -= delta.x;
                    new_window_size.height += delta.y;
                }
                AppEvent::WindowResizeBottomRight => {
                    new_window_size.width += delta.x;
                    new_window_size.height += delta.y;
                }
                AppEvent::WindowResizeBottom => new_window_size.height += delta.y,
                AppEvent::WindowResizeLeft => {
                    new_window_position.x += delta.x;
                    new_window_size.width -= delta.x;
                }
                AppEvent::WindowResizeRight => new_window_size.width += delta.x,
                _ => {}
            }

            wrapper.set_window_size(PhysicalSize::new(
                new_window_size.width as u32,
                new_window_size.height as u32,
            ));
            wrapper.set_window_position(PhysicalPosition::new(
                new_window_position.x as i32,
                new_window_position.y as i32,
            ));
        }
    }

    pub fn draw_window_resize(
        &mut self,
        window_size: PhysicalSize<u32>,
        frame_builder: &mut FrameBuilder,
    ) {
        let builder = &mut frame_builder.builder;
        let clip_id = builder.define_clip_rounded_rect(
            &frame_builder.space_and_clip,
            ComplexClipRegion::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(5.0, 5.0),
                    LayoutSize::new(
                        window_size.width as f32 - 10.0,
                        window_size.height as f32 - 10.0,
                    ),
                ),
                BorderRadius::uniform(5.0),
                ClipMode::ClipOut,
            ),
        );
        let space_and_clip = SpaceAndClipInfo {
            spatial_id: frame_builder.space_and_clip.spatial_id,
            clip_id,
        };

        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(20.0, 0.0),
                    LayoutSize::new(window_size.width as f32 - 40.0, 5.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeTop.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(20.0, window_size.height as f32 - 5.0),
                    LayoutSize::new(window_size.width as f32 - 40.0, 5.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeBottom.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(0.0, 20.0),
                    LayoutSize::new(5.0, window_size.height as f32 - 40.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeLeft.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(window_size.width as f32 - 5.0, 20.0),
                    LayoutSize::new(5.0, window_size.height as f32 - 40.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeRight.into(), 0),
        );

        // corners
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(0.0, 0.0),
                    LayoutSize::new(20.0, 20.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeTopLeft.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(window_size.width as f32 - 20.0, 0.0),
                    LayoutSize::new(20.0, 20.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeTopRight.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(0.0, window_size.height as f32 - 20.0),
                    LayoutSize::new(20.0, 20.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeBottomLeft.into(), 0),
        );
        builder.push_hit_test(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(
                        window_size.width as f32 - 20.0,
                        window_size.height as f32 - 20.0,
                    ),
                    LayoutSize::new(20.0, 20.0),
                ),
                space_and_clip,
            ),
            (AppEvent::WindowResizeBottomRight.into(), 0),
        );
    }
}
