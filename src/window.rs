use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use gleam::gl;
use glutin::{NotCurrent, PossiblyCurrent};
use webrender::api::units::*;
use webrender::api::*;
use winit::dpi::{PhysicalPosition, PhysicalSize};

struct Notifier {
    events_proxy: winit::event_loop::EventLoopProxy<()>,
}

impl Notifier {
    fn new(events_proxy: winit::event_loop::EventLoopProxy<()>) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        self.events_proxy.send_event(()).ok();
    }

    fn new_frame_ready(
        &self,
        _: DocumentId,
        _scrolled: bool,
        _composite_needed: bool,
        _render_time_ns: Option<u64>,
    ) {
        self.wake_up();
    }
}

pub struct WindowOptions {
    pub name: &'static str,
    pub size: PhysicalSize<u32>,
    pub icon: Option<&'static str>,
    pub min_size: Option<PhysicalSize<u32>>,
    pub max_size: Option<PhysicalSize<u32>>,
    pub position: Option<PhysicalPosition<i32>>,
    pub resizable: bool,
    pub fullscreen: Option<winit::window::Fullscreen>,
    pub maximized: bool,
    pub visible: bool,
    pub transparent: bool,
    pub decorations: bool,
    pub always_on_top: bool,
}

impl WindowOptions {
    pub fn new(name: &'static str, width: u32, height: u32, icon: Option<&'static str>) -> Self {
        Self {
            name,
            size: PhysicalSize::new(width, height),
            icon,
            min_size: None,
            max_size: None,
            position: None,
            resizable: true,
            fullscreen: None,
            maximized: false,
            visible: true,
            transparent: false,
            decorations: true,
            always_on_top: false,
        }
    }
}

pub struct Window {
    pub events_loop: winit::event_loop::EventLoop<()>,
    pub context: Option<glutin::WindowedContext<NotCurrent>>,
    pub renderer: webrender::Renderer,
    pub name: &'static str,
    pipeline_id: PipelineId,
    pub document_id: DocumentId,
    epoch: Epoch,
    pub api: RenderApi,
    pub font_key_hashmap: HashMap<&'static str, Rc<FontKey>>,
    pub font_instance_key_hashmap: HashMap<Rc<FontInstanceKey>, Rc<FontKey>>,
}

impl Window {
    pub fn new(
        window_options: WindowOptions,
        clear_color: Option<ColorF>,
        document_layer: DocumentLayer,
    ) -> Self {
        let events_loop = winit::event_loop::EventLoop::new();
        let mut window_builder = winit::window::WindowBuilder::new()
            .with_title(window_options.name)
            .with_inner_size(window_options.size)
            .with_resizable(window_options.resizable)
            .with_fullscreen(window_options.fullscreen)
            .with_maximized(window_options.maximized)
            .with_visible(window_options.visible)
            .with_transparent(window_options.transparent)
            .with_decorations(window_options.decorations)
            .with_always_on_top(window_options.always_on_top);

        if let Some(pathname) = window_options.icon {
            window_builder = window_builder.with_window_icon(Self::load_icon(pathname));
        }
        if let Some(min_size) = window_options.min_size {
            window_builder = window_builder.with_min_inner_size(min_size);
        }
        if let Some(max_size) = window_options.max_size {
            window_builder = window_builder.with_max_inner_size(max_size);
        }

        let context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .with_vsync(true)
            .with_double_buffer(Some(true))
            .with_multisampling(4)
            .build_windowed(window_builder, &events_loop)
            .unwrap();

        let context = unsafe { context.make_current().unwrap() };

        let gl = match context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let opts = webrender::RendererOptions {
            clear_color,
            ..webrender::RendererOptions::default()
        };

        let device_size = {
            let size = context.window().inner_size();
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
        let (renderer, sender) =
            webrender::Renderer::new(gl.clone(), notifier, opts, None, device_size).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size, document_layer);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        Window {
            events_loop,
            context: Some(unsafe { context.make_not_current().unwrap() }),
            renderer,
            name: window_options.name,
            epoch,
            pipeline_id,
            document_id,
            api,
            font_key_hashmap: HashMap::new(),
            font_instance_key_hashmap: HashMap::new(),
        }
    }

    pub fn load_font_file(&mut self, name: &'static str, pathname: &str) {
        let mut txn = Transaction::new();

        let font_key = self.api.generate_font_key();
        txn.add_raw_font(font_key, load_file(pathname), 0);

        self.font_key_hashmap.insert(name, Rc::new(font_key));
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn load_font(&mut self, name: &'static str, font_size: Au) -> Font {
        let mut txn = Transaction::new();

        let font = Font::new(
            self.api.generate_font_instance_key(),
            self.font_key_hashmap[&name].clone(),
            font_size,
        );
        txn.add_font_instance(
            font.instance_key,
            *font.key,
            font_size,
            None,
            None,
            Vec::new(),
        );

        self.api.send_transaction(self.document_id, txn);

        font
    }

    pub fn unload_font(&mut self, font: Font) {
        let mut txn = Transaction::new();

        txn.delete_font_instance(font.instance_key);

        self.api.send_transaction(self.document_id, txn);
    }

    pub fn build_frame(&mut self) -> FrameBuilder {
        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        let device_pixel_ratio = context.window().scale_factor() as f32;
        let device_size = {
            let size = context.window().inner_size();
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let layout_size = device_size.to_f32() / euclid::Scale::new(device_pixel_ratio);
        let txn = Transaction::new();
        let builder = DisplayListBuilder::new(self.pipeline_id, layout_size);
        let space_and_clip = SpaceAndClipInfo::root_scroll(self.pipeline_id);

        let bounds = LayoutRect::from_size(layout_size);

        FrameBuilder {
            context,
            device_size,
            layout_size,
            txn,
            builder,
            space_and_clip,
            bounds,
        }
    }

    pub fn render_frame(&mut self, mut frame_builder: FrameBuilder) {
        frame_builder.txn.set_display_list(
            self.epoch,
            None,
            frame_builder.layout_size,
            frame_builder.builder.finalize(),
            true,
        );
        frame_builder.txn.set_root_pipeline(self.pipeline_id);
        frame_builder.txn.generate_frame();

        self.api
            .send_transaction(self.document_id, frame_builder.txn);
        self.renderer.update();
        self.renderer.render(frame_builder.device_size).unwrap();

        frame_builder.context.swap_buffers().ok();

        self.context = Some(unsafe { frame_builder.context.make_not_current().unwrap() });
    }

    fn load_icon(pathname: &'static str) -> Option<winit::window::Icon> {
        let decoder = png::Decoder::new(File::open(pathname).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bytes = &buf[..info.buffer_size()];

        if let png::ColorType::Rgba = info.color_type {
            winit::window::Icon::from_rgba(bytes.to_vec(), info.width, info.height).ok()
        } else {
            None
        }
    }

    pub fn deinit(self) {
        self.renderer.deinit();
    }
}

pub struct Font {
    pub instance_key: FontInstanceKey,
    pub key: Rc<FontKey>,
    pub size: Au,
}

impl Font {
    pub fn new(font_instance_key: FontInstanceKey, font_key: Rc<FontKey>, font_size: Au) -> Self {
        Self {
            instance_key: font_instance_key,
            key: font_key,
            size: font_size,
        }
    }

    pub fn push_text(
        &self,
        frame_builder: &mut FrameBuilder,
        api: &RenderApi,
        text: &'static str,
        color: ColorF,
        tab_size_option: Option<f32>,
        position: LayoutPoint,
    ) -> LayoutRect {
        let tab_size = if let Some(tab_size) = tab_size_option {
            tab_size
        } else {
            4.0
        };
        let glyph_indices: Vec<u32> = api
            .get_glyph_indices(*self.key, text)
            .into_iter()
            .flatten()
            .collect();
        let glyph_dimension_options =
            api.get_glyph_dimensions(self.instance_key, glyph_indices.clone());
        let mut glyph_instances = vec![];
        let mut glyph_position = position.clone();
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
                match text.get(index..index + 1) {
                    Some(" ") => {
                        glyph_position += LayoutSize::new(char_width_mean, 0.0);
                        glyph_size += LayoutSize::new(char_width_mean, 0.0);
                    }
                    Some("\t") => {
                        glyph_position += LayoutSize::new(char_width_mean * tab_size, 0.0);
                        glyph_size += LayoutSize::new(char_width_mean * tab_size, 0.0);
                    }
                    Some("\n") => {
                        glyph_position = position.clone();
                        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px() * line_count);
                        glyph_size += LayoutSize::new(0.0, self.size.to_f32_px());
                        line_count += 1.0;
                    }
                    Some("\r") => {
                        glyph_position = position.clone();
                        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px() * line_count);
                        glyph_size += LayoutSize::new(0.0, self.size.to_f32_px());
                        line_count += 1.0;
                    }
                    Some(&_) | None => {}
                }
            }
        }

        glyph_position += LayoutSize::new(0.0, self.size.to_f32_px());

        let text_bounds = LayoutRect::new(position, glyph_size.to_vector().to_size());

        frame_builder.builder.push_text(
            &CommonItemProperties::new(text_bounds, frame_builder.space_and_clip),
            text_bounds,
            &glyph_instances,
            self.instance_key,
            color,
            None,
        );

        text_bounds
    }
}

pub struct FrameBuilder {
    context: glutin::WindowedContext<PossiblyCurrent>,
    pub device_size: webrender::euclid::Size2D<i32, units::DevicePixel>,
    layout_size: webrender::euclid::Size2D<f32, units::LayoutPixel>,
    pub txn: Transaction,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

fn load_file(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    buffer
}
