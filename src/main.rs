mod window;

use std::time::Duration;

use util::time::Timer;
use webrender::api::units::*;
use webrender::api::*;
use webrender::DebugFlags;
use winit::platform::run_return::EventLoopExtRunReturn;

fn main() {
    let mut window_options = window::WindowOptions::new("test", 800, 600, Some("./ui/icon.png"));
    window_options.transparent = true;

    let mut window = window::Window::new(
        window_options,
        Some(ColorF::from(ColorU::new(33, 33, 33, 250))),
        0,
    );

    window.load_font_file("OpenSans", "./ui/font/OpenSans.ttf");

    run_ui(&mut window);

    window.deinit();
}

fn run_ui(window: &mut window::Window) {
    let mut timer = Timer::new(Duration::from_micros(3333));
    let font = window.load_font("OpenSans", units::Au::from_f64_px(32.0));

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

        let text_bounds =
            LayoutRect::new(LayoutPoint::new(100.0, 50.0), LayoutSize::new(700.0, 200.0));
        let glyphs = vec![
            GlyphInstance {
                index: 48,
                point: LayoutPoint::new(100.0, 100.0),
            },
            GlyphInstance {
                index: 68,
                point: LayoutPoint::new(150.0, 100.0),
            },
            GlyphInstance {
                index: 80,
                point: LayoutPoint::new(200.0, 100.0),
            },
            GlyphInstance {
                index: 82,
                point: LayoutPoint::new(250.0, 100.0),
            },
            GlyphInstance {
                index: 81,
                point: LayoutPoint::new(300.0, 100.0),
            },
            GlyphInstance {
                index: 3,
                point: LayoutPoint::new(350.0, 100.0),
            },
            GlyphInstance {
                index: 86,
                point: LayoutPoint::new(400.0, 100.0),
            },
            GlyphInstance {
                index: 79,
                point: LayoutPoint::new(450.0, 100.0),
            },
            GlyphInstance {
                index: 72,
                point: LayoutPoint::new(500.0, 100.0),
            },
            GlyphInstance {
                index: 83,
                point: LayoutPoint::new(550.0, 100.0),
            },
            GlyphInstance {
                index: 87,
                point: LayoutPoint::new(600.0, 100.0),
            },
            GlyphInstance {
                index: 17,
                point: LayoutPoint::new(650.0, 100.0),
            },
        ];

        builder.push_text(
            &CommonItemProperties::new(text_bounds, frame_builder.space_and_clip),
            text_bounds,
            &glyphs,
            font,
            ColorF::new(1.0, 1.0, 0.0, 1.0),
            None,
        );

        frame_builder.builder.pop_stacking_context();

        window.render_frame(frame_builder);
        timer.wait();
    }
}
