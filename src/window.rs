pub mod ext;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::time::Duration;

use gleam::gl;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};
use png::{ColorType, Decoder};
use util::time::Timer;
use webrender::api::units::*;
use webrender::api::*;
use webrender::{DebugFlags, Renderer, RendererOptions};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{Icon, WindowBuilder};

struct Notifier {
    events_proxy: EventLoopProxy<()>,
}

impl Notifier {
    fn new(events_proxy: EventLoopProxy<()>) -> Notifier {
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
    pub title: &'static str,
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
    pub fn new(title: &'static str, width: u32, height: u32, icon: Option<&'static str>) -> Self {
        Self {
            title,
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
    pub title: &'static str,
    pub context: Rc<WindowedContext<PossiblyCurrent>>,
    pub renderer: Renderer,
    pub pipeline_id: PipelineId,
    pub document_id: DocumentId,
    epoch: Epoch,
    pub api: RenderApi,
    font_key_hashmap: HashMap<&'static str, Rc<FontKey>>,
    device_size: DeviceIntSize,
}

impl WindowWrapper {
    fn new(
        title: &'static str,
        context: Rc<WindowedContext<PossiblyCurrent>>,
        renderer: Renderer,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        epoch: Epoch,
        api: RenderApi,
        font_key_hashmap: HashMap<&'static str, Rc<FontKey>>,
    ) -> Self {
        let window_size = context.window().inner_size();

        Self {
            title,
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
    event_loop: EventLoop<()>,
    pub wrapper: WindowWrapper,
    window: Box<dyn WindowTrait>,
}

impl Window {
    pub fn new(
        window_options: WindowOptions,
        clear_color: Option<ColorF>,
        document_layer: DocumentLayer,
    ) -> Self {
        let event_loop = EventLoop::new();
        let window = DefaultWindow::new();
        let mut window_builder = WindowBuilder::new()
            .with_title(window_options.title)
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

        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
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
            Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            Api::WebGl => unimplemented!(),
        };

        let opts = RendererOptions {
            clear_color,
            ..RendererOptions::default()
        };

        let device_size = {
            let size = context.window().inner_size();
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let notifier = Box::new(Notifier::new(event_loop.create_proxy()));
        let (renderer, sender) = Renderer::new(gl, notifier, opts, None, device_size).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size, document_layer);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        Window {
            event_loop,
            wrapper: WindowWrapper::new(
                window_options.title,
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
    }

    pub fn run(&mut self) {
        let mut timer = Timer::new(Duration::from_millis(12));

        loop {
            let mut exit = false;

            self.event_loop
                .run_return(|global_event, _event_loop_window_target, control_flow| {
                    *control_flow = ControlFlow::Exit;

                    match global_event {
                        winit::event::Event::WindowEvent { event, .. } => match event {
                            WindowEvent::CloseRequested => {
                                exit = true;
                            }
                            WindowEvent::Resized(size) => {
                                self.wrapper.redraw(&mut self.window, Some(size));
                            }
                            WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(key),
                                        ..
                                    },
                                ..
                            } => match key {
                                VirtualKeyCode::P => {
                                    println!("set flags {}", self.wrapper.title);
                                    self.wrapper.api.send_debug_cmd(DebugCommand::SetFlags(
                                        DebugFlags::PROFILER_DBG,
                                    ));
                                }
                                _ => {}
                            },
                            WindowEvent::CursorMoved { position, .. } => self
                                .window
                                .on_event(Event::MousePosition(position), &mut self.wrapper),
                            WindowEvent::MouseInput {
                                state: ElementState::Pressed,
                                button,
                                ..
                            } => self
                                .window
                                .on_event(Event::MousePressed(button), &mut self.wrapper),
                            WindowEvent::MouseInput {
                                state: ElementState::Released,
                                button,
                                ..
                            } => self
                                .window
                                .on_event(Event::MouseReleased(button), &mut self.wrapper),
                            _ => {}
                        },
                        _ => {}
                    };
                });

            if exit || self.window.should_exit() {
                break;
            }

            self.wrapper.redraw(&mut self.window, None);

            timer.wait();
        }
    }

    fn load_icon(pathname: &'static str) -> Option<Icon> {
        let decoder = Decoder::new(File::open(pathname).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bytes = &buf[..info.buffer_size()];

        if let ColorType::Rgba = info.color_type {
            Icon::from_rgba(bytes.to_vec(), info.width, info.height).ok()
        } else {
            None
        }
    }

    pub fn deinit(self) {
        self.wrapper.renderer.deinit();
    }
}

pub struct FrameBuilder {
    pub device_size: DeviceIntSize,
    layout_size: LayoutSize,
    pub builder: DisplayListBuilder,
    pub space_and_clip: SpaceAndClipInfo,
    pub bounds: LayoutRect,
}

impl FrameBuilder {
    fn new(
        device_size: DeviceIntSize,
        layout_size: LayoutSize,
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
    MousePosition(PhysicalPosition<f64>),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
}

pub trait WindowTrait {
    fn on_event(&mut self, event: Event, window: &mut WindowWrapper);

    fn should_exit(&self) -> bool;

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
    fn on_event(&mut self, _: Event, _: &mut WindowWrapper) {}

    fn should_exit(&self) -> bool {
        false
    }

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
