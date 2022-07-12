use std::sync::{Arc, Mutex};

use util::thread::MutexTrait;
use webrender::api::units::{Au, LayoutPoint, LayoutRect, LayoutSize};
use webrender::api::{
    ColorF, CommonItemProperties, DisplayListBuilder, DocumentId, FontInstanceKey, FontKey,
    GlyphDimensions, GlyphInstance, GlyphOptions, SpaceAndClipInfo,
};
use webrender::render_api::{RenderApi, Transaction};

pub struct Font {
    pub instance_key: FontInstanceKey,
    pub key: FontKey,
    pub size: Au,
    api_mutex: Arc<Mutex<RenderApi>>,
    document_id: DocumentId,
}

impl Font {
    pub fn new(
        font_key: FontKey,
        font_size: Au,
        api_mutex: Arc<Mutex<RenderApi>>,
        document_id: DocumentId,
    ) -> Self {
        let font_instance_key = {
            let mut api = api_mutex.lock_poisoned();
            let font_instance_key = api.generate_font_instance_key();
            let mut txn = Transaction::new();

            txn.add_font_instance(
                font_instance_key,
                font_key,
                font_size.to_f32_px(),
                None,
                None,
                Vec::new(),
            );
            api.send_transaction(document_id, txn);

            font_instance_key
        };

        Self {
            instance_key: font_instance_key,
            key: font_key,
            size: font_size,
            api_mutex,
            document_id,
        }
    }

    pub fn create_text(&self, text: String, tab_size_option: Option<f32>) -> Text {
        let api = self.api_mutex.lock_poisoned();
        let char_vec: Vec<char> = text.chars().collect();
        let tab_size = if let Some(tab_size) = tab_size_option {
            tab_size
        } else {
            4.0
        };
        let glyph_indices: Vec<u32> = api
            .get_glyph_indices(self.key, text.as_str())
            .into_iter()
            .flatten()
            .collect();
        let glyph_dimension_options =
            api.get_glyph_dimensions(self.instance_key, glyph_indices.clone());
        let mut glyph_size = LayoutSize::new(0.0, self.size.to_f32_px());
        let mut char_width_mean = 0.0;
        let mut char_width_count = 0;
        let mut max_line_height = 0.0f32;

        for glyph_dimension_option in glyph_dimension_options.clone() {
            if let Some(glyph_dimension) = glyph_dimension_option {
                char_width_mean += glyph_dimension.width as f32;
                char_width_count += 1;
            }
        }

        char_width_mean /= char_width_count as f32;

        for index in 0..glyph_indices.len() {
            if let Some(glyph_dimension) = glyph_dimension_options[index] {
                glyph_size += LayoutSize::new(glyph_dimension.advance, 0.0);
                max_line_height = max_line_height.max(
                    self.size.to_f32_px() - glyph_dimension.top as f32
                        + glyph_dimension.height as f32,
                );
            } else {
                match char_vec[index] {
                    ' ' => glyph_size += LayoutSize::new(char_width_mean, 0.0),
                    '\t' => glyph_size += LayoutSize::new(char_width_mean * tab_size, 0.0),
                    '\n' | '\r' => {
                        glyph_size += LayoutSize::new(0.0, self.size.to_f32_px());
                        max_line_height = 0.0;
                    }
                    _ => {}
                }
            }
        }

        // add extra height on the last line for letters like "g" which goes further down
        if self.size.to_f32_px() != max_line_height {
            glyph_size += LayoutSize::new(0.0, max_line_height - self.size.to_f32_px())
        }

        Text::new(
            glyph_size,
            char_vec,
            glyph_indices,
            glyph_dimension_options,
            self.size,
            self.instance_key,
            char_width_mean,
            tab_size,
        )
    }

    pub fn unload(&mut self) {
        let mut txn = Transaction::new();

        txn.delete_font_instance(self.instance_key);

        self.api_mutex
            .lock_poisoned()
            .send_transaction(self.document_id, txn);
    }
}

pub struct Text {
    pub size: LayoutSize,
    pub char_vec: Vec<char>,
    pub glyph_indices: Vec<u32>,
    pub glyph_dimension_options: Vec<Option<GlyphDimensions>>,
    pub font_size: Au,
    instance_key: FontInstanceKey,
    char_width_mean: f32,
    tab_size: f32,
}

impl Text {
    fn new(
        size: LayoutSize,
        char_vec: Vec<char>,
        glyph_indices: Vec<u32>,
        glyph_dimension_options: Vec<Option<GlyphDimensions>>,
        font_size: Au,
        instance_key: FontInstanceKey,
        char_width_mean: f32,
        tab_size: f32,
    ) -> Self {
        Self {
            size,
            char_vec,
            glyph_indices,
            glyph_dimension_options,
            font_size,
            instance_key,
            char_width_mean,
            tab_size,
        }
    }

    pub fn push_text(
        &self,
        builder: &mut DisplayListBuilder,
        space_and_clip: SpaceAndClipInfo,
        position: LayoutPoint,
        color: ColorF,
        glyph_options: Option<GlyphOptions>,
    ) {
        let mut glyph_instances = vec![];
        let mut glyph_position = position + LayoutSize::new(0.0, self.font_size.to_f32_px());
        let mut line_count = 1.0;

        for (index, glyph_indice) in self.glyph_indices.iter().enumerate() {
            if let Some(glyph_dimension) = self.glyph_dimension_options[index] {
                glyph_instances.push(GlyphInstance {
                    index: *glyph_indice,
                    point: glyph_position,
                });
                glyph_position += LayoutSize::new(glyph_dimension.advance, 0.0);
            } else {
                match self.char_vec[index] {
                    ' ' => {
                        glyph_position += LayoutSize::new(self.char_width_mean, 0.0);
                    }
                    '\t' => {
                        glyph_position +=
                            LayoutSize::new(self.char_width_mean * self.tab_size, 0.0);
                    }
                    '\n' | '\r' => {
                        glyph_position = position;
                        glyph_position +=
                            LayoutSize::new(0.0, self.font_size.to_f32_px() * (line_count + 1.0));
                        line_count += 1.0;
                    }
                    _ => {}
                }
            }
        }

        let text_bounds =
            LayoutRect::from_origin_and_size(position, self.size.to_vector().to_size());

        builder.push_text(
            &CommonItemProperties::new(text_bounds, space_and_clip),
            text_bounds,
            &glyph_instances,
            self.instance_key,
            color,
            glyph_options,
        );
    }
}
