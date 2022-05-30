pub mod ext;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::time::Duration;

use gleam::gl;
use glutin::PossiblyCurrent;
use util::time::Timer;
use webrender::api::units::*;
use webrender::api::*;
use webrender::DebugFlags;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::platform::run_return::EventLoopExtRunReturn;

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
    pub fn new(width: u32, height: u32, icon: Option<&'static str>) -> Self {
        Self {
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

pub struct WindowWrapper {
    pub context: Rc<glutin::WindowedContext<PossiblyCurrent>>,
    pub renderer: webrender::Renderer,
    pub pipeline_id: PipelineId,
    pub document_id: DocumentId,
    epoch: Epoch,
    pub api: RenderApi,
    font_key_hashmap: HashMap<&'static str, Rc<FontKey>>,
    device_size: DeviceIntSize,
}

impl WindowWrapper {
    fn new(
        context: Rc<glutin::WindowedContext<PossiblyCurrent>>,
        renderer: webrender::Renderer,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        epoch: Epoch,
        api: RenderApi,
        font_key_hashmap: HashMap<&'static str, Rc<FontKey>>,
    ) -> Self {
        let window_size = context.window().inner_size();

        Self {
            context,
            renderer,
            pipeline_id,
            document_id,
            epoch,
            api,
            font_key_hashmap,
            device_size: DeviceIntSize::new(window_size.width as i32, window_size.height as i32),
        }
    }

    pub fn resize_window(&self, size: PhysicalSize<u32>) {
        self.context.resize(size);
        self.api.set_document_view(
            self.document_id,
            DeviceIntRect::new(
                DeviceIntPoint::zero(),
                DeviceIntSize::new(size.width as i32, size.height as i32),
            ),
            self.context.window().scale_factor() as f32,
        );
    }

    pub fn get_window_size(&self) -> PhysicalSize<u32> {
        self.context.window().inner_size()
    }

    fn init_frame_builder(&mut self) -> FrameBuilder {
        let window_size = self.get_window_size();

        self.device_size = DeviceIntSize::new(window_size.width as i32, window_size.height as i32);

        let layout_size = self.device_size.to_f32()
            / euclid::Scale::new(self.context.window().scale_factor() as f32);
        let builder = DisplayListBuilder::new(self.pipeline_id, layout_size);
        let space_and_clip = SpaceAndClipInfo::root_scroll(self.pipeline_id);
        let bounds = LayoutRect::from_size(layout_size);

        FrameBuilder::new(
            self.device_size,
            layout_size,
            builder,
            space_and_clip,
            bounds,
        )
    }

    fn redraw(
        &mut self,
        window: &mut Box<dyn WindowTrait>,
        new_window_size: Option<PhysicalSize<u32>>,
    ) {
        let mut resized = false;

        if let Some(size) = new_window_size {
            self.resize_window(size);

            resized = true;
        }

        if window.should_rerender() || resized {
            let mut frame_builder = self.init_frame_builder();

            window.render(&mut frame_builder, self);

            let mut txn = Transaction::new();

            txn.set_display_list(
                self.epoch,
                None,
                frame_builder.layout_size,
                frame_builder.builder.finalize(),
                true,
            );
            txn.set_root_pipeline(self.pipeline_id);
            txn.generate_frame();

            self.api.send_transaction(self.document_id, txn);
        }

        self.renderer.update();
        self.renderer.render(self.device_size).unwrap();
        self.context.swap_buffers().ok();
    }

    pub fn load_font_file(&mut self, name: &'static str, pathname: &str) {
        let mut txn = Transaction::new();

        let font_key = self.api.generate_font_key();
        txn.add_raw_font(font_key, load_file(pathname), 0);

        self.font_key_hashmap.insert(name, Rc::new(font_key));
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn load_font(&self, name: &'static str, font_size: Au) -> Font {
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
}

pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    pub wrapper: WindowWrapper,
    window: Box<dyn WindowTrait>,
}

impl Window {
    pub fn new(
        window_options: WindowOptions,
        clear_color: Option<ColorF>,
        document_layer: DocumentLayer,
    ) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = DefaultWindow::new();
        let mut window_builder = winit::window::WindowBuilder::new()
            .with_title(window.get_title())
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
            .build_windowed(window_builder, &event_loop)
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
        let notifier = Box::new(Notifier::new(event_loop.create_proxy()));
        let (renderer, sender) =
            webrender::Renderer::new(gl, notifier, opts, None, device_size).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size, document_layer);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        Window {
            event_loop,
            wrapper: WindowWrapper::new(
                Rc::new(context),
                renderer,
                pipeline_id,
                document_id,
                epoch,
                api,
                HashMap::new(),
            ),
            window,
        }
    }

    pub fn set_window(&mut self, window: Box<dyn WindowTrait>) {
        self.window = window;
        self.wrapper
            .context
            .window()
            .set_title(self.window.get_title());
    }

    pub fn run(&mut self) {
        let mut timer = Timer::new(Duration::from_millis(12));
        let mut exit = false;

        while !exit {
            self.event_loop
                .run_return(|global_event, _event_loop_window_target, control_flow| {
                    match global_event {
                        winit::event::Event::WindowEvent { event, .. } => match event {
                            winit::event::WindowEvent::CloseRequested => {
                                self.window.on_event(Event::CloseRequest, &mut self.wrapper);

                                exit = true;
                            }
                            winit::event::WindowEvent::Resized(size) => {
                                self.window
                                    .on_event(Event::Resized(size), &mut self.wrapper);
                                self.wrapper.redraw(&mut self.window, Some(size));
                            }
                            winit::event::WindowEvent::KeyboardInput {
                                input:
                                    winit::event::KeyboardInput {
                                        state: winit::event::ElementState::Pressed,
                                        virtual_keycode: Some(key),
                                        ..
                                    },
                                ..
                            } => match key {
                                winit::event::VirtualKeyCode::Escape => {
                                    self.window.on_event(Event::CloseRequest, &mut self.wrapper);

                                    exit = true;
                                }
                                winit::event::VirtualKeyCode::P => {
                                    println!("set flags {}", self.window.get_title());
                                    self.wrapper.api.send_debug_cmd(DebugCommand::SetFlags(
                                        DebugFlags::PROFILER_DBG,
                                    ));
                                }
                                _ => {}
                            },
                            winit::event::WindowEvent::MouseInput {
                                state: winit::event::ElementState::Pressed,
                                button: winit::event::MouseButton::Left,
                                ..
                            } => {
                                self.wrapper.context.window().drag_window().unwrap();
                            }
                            _ => {}
                        },
                        _ => {}
                    };

                    *control_flow = winit::event_loop::ControlFlow::Exit;
                });

            self.wrapper.redraw(&mut self.window, None);

            timer.wait();
        }
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
        self.wrapper.renderer.deinit();
    }
}

pub struct FrameBuilder {
    pub device_size: webrender::euclid::Size2D<i32, units::DevicePixel>,
    layout_size: webrender::euclid::Size2D<f32, units::LayoutPixel>,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    fn new(
        device_size: webrender::euclid::Size2D<i32, units::DevicePixel>,
        layout_size: webrender::euclid::Size2D<f32, units::LayoutPixel>,
        builder: DisplayListBuilder,
        space_and_clip: SpaceAndClipInfo,
        bounds: LayoutRect,
    ) -> Self {
        Self {
            device_size,
            layout_size,
            builder,
            space_and_clip,
            bounds,
        }
    }
}

pub struct Font {
    pub instance_key: FontInstanceKey,
    pub key: Rc<FontKey>,
    pub size: Au,
}

impl Font {
    fn new(font_instance_key: FontInstanceKey, font_key: Rc<FontKey>, font_size: Au) -> Self {
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
        let char_iterator: Vec<char> = text.chars().collect();
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

#[derive(Clone, Copy)]
pub enum Event {
    CloseRequest,
    Resized(PhysicalSize<u32>),
}

pub trait WindowTrait {
    fn get_title(&self) -> &'static str;

    fn on_event(&mut self, event: Event, window: &mut WindowWrapper);

    fn should_rerender(&self) -> bool;

    fn render(&mut self, frame_builder: &mut FrameBuilder, window: &mut WindowWrapper);
}

struct DefaultWindow {}

impl DefaultWindow {
    fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl WindowTrait for DefaultWindow {
    fn get_title(&self) -> &'static str {
        ""
    }

    fn on_event(&mut self, _: Event, _: &mut WindowWrapper) {}

    fn should_rerender(&self) -> bool {
        false
    }

    fn render(&mut self, _: &mut FrameBuilder, _: &mut WindowWrapper) {}
}

fn load_file(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    buffer
}
