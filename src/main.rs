mod window;

use std::time::Duration;

use util::time::Timer;
use webrender::api::units::*;
use webrender::api::*;
use webrender::DebugFlags;
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use winit::platform::run_return::EventLoopExtRunReturn;

fn main() {
    let mut window_options = window::WindowOptions::new("test", 800, 600, Some("./ui/icon.png"));
    window_options.transparent = true;

    let mut window = window::Window::new(
        window_options,
        Some(ColorF::from(ColorU::new(33, 33, 33, 240))),
        0,
    );

    {
        // add background blur effect on windows and macos
        let context = unsafe { window.context.take().unwrap().make_current().unwrap() };

        #[cfg(target_os = "windows")]
        apply_blur(&context.window(), None).ok();

        #[cfg(target_os = "macos")]
        apply_vibrancy(&context.window(), NSVisualEffectMaterial::AppearanceBased).ok();

        window.context = Some(unsafe { context.make_not_current().unwrap() });
    }

    window.load_font_file("OpenSans", "./ui/font/OpenSans.ttf");

    run_ui(&mut window);

    window.deinit();
}

fn run_ui(window: &mut window::Window) {
    let mut timer = Timer::new(Duration::from_micros(3333));
    let font = window.load_font("OpenSans", units::Au::from_f32_px(32.0));

    loop {
        let mut do_exit = false;

        window
            .events_loop
            .run_return(|global_event, _elwt, control_flow| {
                *control_flow = winit::event_loop::ControlFlow::Exit;
                match global_event {
                    winit::event::Event::WindowEvent { event, .. } => match event {
                        winit::event::WindowEvent::CloseRequested
                        | winit::event::WindowEvent::KeyboardInput {
                            input:
                                winit::event::KeyboardInput {
                                    virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => do_exit = true,
                        winit::event::WindowEvent::KeyboardInput {
                            input:
                                winit::event::KeyboardInput {
                                    state: winit::event::ElementState::Pressed,
                                    virtual_keycode: Some(winit::event::VirtualKeyCode::P),
                                    ..
                                },
                            ..
                        } => {
                            println!("set flags {}", window.name);
                            window
                                .api
                                .send_debug_cmd(DebugCommand::SetFlags(DebugFlags::PROFILER_DBG))
                        }
                        _ => {}
                    },
                    _ => {}
                }
            });

        if do_exit {
            break;
        }

        let mut frame_builder = window.build_frame();
        let builder = &mut frame_builder.builder;

        builder.push_simple_stacking_context(
            frame_builder.bounds.min(),
            frame_builder.space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    LayoutPoint::new(100.0, 200.0),
                    LayoutSize::new(100.0, 200.0),
                ),
                frame_builder.space_and_clip,
            ),
            ColorF::new(0.0, 1.0, 0.0, 1.0),
        );

        font.push_text(
            &mut frame_builder,
            &window.api,
            "Salut comment\n ça\r\tva",
            ColorF::new(1.0, 1.0, 0.0, 1.0),
            None,
            LayoutPoint::new(100.0, 50.0),
        );

        frame_builder.builder.pop_stacking_context();
        window.render_frame(frame_builder);
        timer.wait();
    }
}
