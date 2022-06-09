use std::cell::RefCell;
use std::rc::Rc;

use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    ColorF, CommonItemProperties, DisplayListBuilder, DocumentId, FontInstanceKey, FontKey,
    GlyphInstance, SpaceAndClipInfo,
};
use webrender::render_api::{RenderApi, Transaction};

use super::ext::LayoutRectExt;

pub struct Font {
    pub instance_key: FontInstanceKey,
    pub key: FontKey,
    pub size: Au,
    api: Rc<RefCell<RenderApi>>,
    document_id: DocumentId,
}

impl Font {
    pub fn new(
        font_key: FontKey,
        font_size: Au,
        api: Rc<RefCell<RenderApi>>,
        document_id: DocumentId,
    ) -> Self {
        let font_instance_key = api.borrow().generate_font_instance_key();
        let mut txn = Transaction::new();

        txn.add_font_instance(
            font_instance_key,
            font_key,
            font_size.to_f32_px(),
            None,
            None,
            Vec::new(),
        );
        api.borrow_mut().send_transaction(document_id, txn);

        Self {
            instance_key: font_instance_key,
            key: font_key,
            size: font_size,
            api,
            document_id,
        }
    }

    pub fn push_text(
        &self,
        builder: &mut DisplayListBuilder,
        text: String,
        color: ColorF,
        position: LayoutPoint,
        space_and_clip: SpaceAndClipInfo,
        tab_size_option: Option<f32>,
    ) -> LayoutRect {
        let char_iterator: Vec<char> = text.chars().collect();
        let tab_size = if let Some(tab_size) = tab_size_option {
            tab_size
        } else {
            4.0
        };
        let glyph_indices: Vec<u32> = self
            .api
            .borrow()
            .get_glyph_indices(self.key, text.as_str())
            .into_iter()
            .flatten()
            .collect();
        let glyph_dimension_options = self
            .api
            .borrow()
            .get_glyph_dimensions(self.instance_key, glyph_indices.clone());
        let mut glyph_instances = vec![];
        let mut glyph_position = position;
        let mut glyph_size = LayoutSize::new(0.0, self.size.to_f32_px());
        let mut line_count = 1.0;
        let mut char_width_mean = 0.0;
        let mut char_width_count = 0;

        for glyph_dimension_option in glyph_dimension_options.clone() {
            if let Some(glyph_dimension) = glyph_dimension_option {
                char_width_mean += glyph_dimension.width as f32;
                char_width_count += 1;
            }
        }

        char_width_mean /= char_width_count as f32;

        for (index, glyph_indice) in glyph_indices.into_iter().enumerate() {
            if let Some(glyph_dimension) = glyph_dimension_options[index] {
                glyph_position += LayoutSize::new(0.0, self.size.to_f32_px());
                glyph_instances.push(GlyphInstance {
                    index: glyph_indice,
                    point: glyph_position,
                });
                glyph_position +=
                    LayoutSize::new(glyph_dimension.advance, -(self.size.to_f32_px()));
                glyph_size += LayoutSize::new(glyph_dimension.advance, 0.0);
            } else {
                match char_iterator[index] {
                    ' ' => {
                        glyph_position += LayoutSize::new(char_width_mean, 0.0);
                        glyph_size += LayoutSize::new(char_width_mean, 0.0);
                    }
                    '\t' => {
                        glyph_position += LayoutSize::new(char_width_mean * tab_size, 0.0);
                        glyph_size += LayoutSize::new(char_width_mean * tab_size, 0.0);
                    }
                    '\n' => {
                        glyph_position = position;
                        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px() * line_count);
                        glyph_size += LayoutSize::new(0.0, self.size.to_f32_px());
                        line_count += 1.0;
                    }
                    '\r' => {
                        glyph_position = position;
                        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px() * line_count);
                        glyph_size += LayoutSize::new(0.0, self.size.to_f32_px());
                        line_count += 1.0;
                    }
                    _ => {}
                }
            }
        }

        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px());

        let text_bounds = LayoutRect::new_with_size(position, glyph_size.to_vector().to_size());

        builder.push_text(
            &CommonItemProperties::new(text_bounds, space_and_clip),
            text_bounds,
            &glyph_instances,
            self.instance_key,
            color,
            None,
        );

        text_bounds
    }

    pub fn unload(&mut self) {
        let mut txn = Transaction::new();

        txn.delete_font_instance(self.instance_key);

        self.api
            .borrow_mut()
            .send_transaction(self.document_id, txn);
    }
}
