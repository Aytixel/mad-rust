pub mod ext;

mod font;
mod frame_builder;
mod notifier;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

pub use font::Font;
pub use frame_builder::FrameBuilder;

use notifier::Notifier;

use gleam::gl;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};
use png::{ColorType, Decoder};
use util::time::Timer;
use webrender::api::units::{Au, DeviceIntPoint, DeviceIntRect, DeviceIntSize, LayoutRect};
use webrender::api::{
    ColorF, DisplayListBuilder, DocumentId, Epoch, FontKey, PipelineId, RenderReasons,
    SpaceAndClipInfo,
};
use webrender::euclid::Scale;
use webrender::render_api::{RenderApi, Transaction};
use webrender::{Renderer, RendererOptions};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{Icon, WindowBuilder};

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
    pub api: Rc<RefCell<RenderApi>>,
    font_key_hashmap: HashMap<&'static str, FontKey>,
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
        mut api: RenderApi,
        font_key_hashmap: HashMap<&'static str, FontKey>,
    ) -> Self {
        let window_size = context.window().inner_size();

        let mut txn = Transaction::new();

        txn.set_root_pipeline(pipeline_id);
        api.send_transaction(document_id, txn);

        Self {
            title,
            context,
            renderer,
            pipeline_id,
            document_id,
            epoch,
            api: Rc::new(RefCell::new(api)),
            font_key_hashmap,
            device_size: DeviceIntSize::new(window_size.width as i32, window_size.height as i32),
        }
    }

    pub fn resize_window(&self, size: PhysicalSize<u32>) {
        self.context.resize(size);
        let mut txn = Transaction::new();

        txn.set_document_view(DeviceIntRect::new(
            DeviceIntPoint::zero(),
            DeviceIntPoint::new(size.width as i32, size.height as i32),
        ));

        self.api
            .borrow_mut()
            .send_transaction(self.document_id, txn);
    }

    pub fn get_window_size(&self) -> PhysicalSize<u32> {
        self.context.window().inner_size()
    }

    fn init_frame_builder(&mut self) -> FrameBuilder {
        let window_size = self.get_window_size();

        self.device_size = DeviceIntSize::new(window_size.width as i32, window_size.height as i32);

        let layout_size =
            self.device_size.to_f32() / Scale::new(self.context.window().scale_factor() as f32);
        let mut builder = DisplayListBuilder::new(self.pipeline_id);
        let space_and_clip = SpaceAndClipInfo::root_scroll(self.pipeline_id);
        let bounds = LayoutRect::from_size(layout_size);

        builder.begin();

        FrameBuilder::new(layout_size, builder, space_and_clip, bounds)
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

        window.animate(self);

        if window.should_redraw() || resized {
            let mut frame_builder = self.init_frame_builder();

            window.redraw(&mut frame_builder, self);

            let mut txn = Transaction::new();

            txn.set_display_list(
                self.epoch,
                None,
                frame_builder.layout_size,
                frame_builder.builder.end(),
            );
            txn.generate_frame(
                0,
                if resized {
                    RenderReasons::RESIZE
                } else {
                    RenderReasons::SCENE
                },
            );

            self.api
                .borrow_mut()
                .send_transaction(self.document_id, txn);
        }

        if window.should_render() || resized {
            self.renderer.update();
            self.renderer.render(self.device_size, 0).unwrap();
            self.context.swap_buffers().ok();
        }
    }

    pub fn load_font_file(&mut self, name: &'static str, pathname: &str) {
        let mut txn = Transaction::new();

        let font_key = self.api.borrow().generate_font_key();
        txn.add_raw_font(font_key, load_file(pathname), 0);

        self.font_key_hashmap.insert(name, font_key);

        self.api
            .borrow_mut()
            .send_transaction(self.document_id, txn);
    }

    pub fn load_font(&mut self, name: &'static str, font_size: Au) -> Font {
        let mut txn = Transaction::new();

        let font = Font::new(
            self.api.borrow().generate_font_instance_key(),
            self.font_key_hashmap[&name].clone(),
            font_size,
            self.api.clone(),
            self.document_id,
        );
        txn.add_font_instance(
            font.instance_key,
            font.key,
            font_size.to_f32_px(),
            None,
            None,
            Vec::new(),
        );

        self.api
            .borrow_mut()
            .send_transaction(self.document_id, txn);

        font
    }

    fn unload_fonts(&mut self) {
        let mut txn = Transaction::new();

        for font_key in self.font_key_hashmap.values() {
            txn.delete_font(*font_key);
        }

        self.api
            .borrow_mut()
            .send_transaction(self.document_id, txn);
    }
}

pub struct Window<T> {
    event_loop: EventLoop<()>,
    pub wrapper: WindowWrapper,
    window: Box<dyn WindowTrait>,
    global_state: Arc<T>,
}

impl<T> Window<T> {
    pub fn new(window_options: WindowOptions, global_state: T, clear_color: ColorF) -> Self {
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
        let (renderer, sender) = Renderer::new(gl, notifier, opts, None).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size);

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
            global_state: Arc::new(global_state),
        }
    }

    pub fn set_window<U: WindowInitTrait<T> + 'static>(&mut self) {
        self.window.unload();
        self.window = U::new(&mut self.wrapper, self.global_state.clone());
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
                            WindowEvent::Resized(size) => {
                                self.wrapper.redraw(&mut self.window, Some(size));
                                self.window
                                    .on_event(Event::Resized(size), &mut self.wrapper)
                            }
                            WindowEvent::CloseRequested => {
                                exit = true;
                            }
                            WindowEvent::CursorMoved { position, .. } => self
                                .window
                                .on_event(Event::MousePosition(position), &mut self.wrapper),
                            WindowEvent::CursorEntered { .. } => {
                                self.window.on_event(Event::MouseEntered, &mut self.wrapper)
                            }
                            WindowEvent::CursorLeft { .. } => {
                                self.window.on_event(Event::MouseLeft, &mut self.wrapper)
                            }
                            WindowEvent::MouseInput { state, button, .. } => match state {
                                ElementState::Pressed => self
                                    .window
                                    .on_event(Event::MousePressed(button), &mut self.wrapper),
                                ElementState::Released => self
                                    .window
                                    .on_event(Event::MouseReleased(button), &mut self.wrapper),
                            },
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

        self.window.unload();
        self.wrapper.unload_fonts();
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

#[derive(Clone, Copy)]
pub enum Event {
    Resized(PhysicalSize<u32>),
    MousePosition(PhysicalPosition<f64>),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    MouseEntered,
    MouseLeft,
}

pub trait WindowInitTrait<T>: WindowTrait {
    fn new(window: &mut WindowWrapper, global_state: Arc<T>) -> Box<dyn WindowTrait>;
}

pub trait WindowTrait {
    fn on_event(&mut self, _event: Event, _window: &mut WindowWrapper) {}

    fn should_exit(&self) -> bool {
        false
    }

    fn should_redraw(&mut self) -> bool {
        false
    }

    fn should_render(&mut self) -> bool {
        false
    }

    fn animate(&mut self, _window: &mut WindowWrapper) {}

    fn redraw(&mut self, _frame_builder: &mut FrameBuilder, _window: &mut WindowWrapper) {}

    fn unload(&mut self) {}
}

struct DefaultWindow {}

impl DefaultWindow {
    fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl WindowTrait for DefaultWindow {}

fn load_file(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();
    let mut buffer = vec![];

    file.read_to_end(&mut buffer).unwrap();
    buffer
}
