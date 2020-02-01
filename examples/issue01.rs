use gouache::{Color, Vec2, Mat2x2, PathBuilder, Frame, Cache, renderers::GlRenderer};

const FRAME: std::time::Duration = std::time::Duration::from_micros(1_000_000 / 60);

fn main() {
    let width = 400.0;
    let height = 200.0;
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(glutin::dpi::LogicalSize::new(width, height))
        .with_title("gouache");
    let context = glutin::ContextBuilder::new()
        .build_windowed(window_builder, &events_loop)
        .unwrap();
    let context = unsafe { context.make_current() }.unwrap();

    gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

    let mut cache = Cache::new();
    let mut renderer = GlRenderer::new();

    let path = PathBuilder::new()
        .move_to(60.2, 80.4)
        .line_to(80.0, 80.4)
        .line_to(89.0, 8.4)
        .line_to(70.2, 8.0)
        .build();
    let path_key = cache.add_path();

    let (mut left, mut right) = (false, false);
    let mut angle = 0.0;
    let mut size = 1.0;

    let mut running = true;
    let mut now = std::time::Instant::now();
    while running {
        let mut frame = Frame::new(&mut cache, &mut renderer, width as _, height as _);

        frame.clear(Color::rgba(1.0, 1.0, 1.0, 1.0));

        if left {
            angle += 0.05;
        } else if right {
            angle -= 0.05;
        }

        let transform = Mat2x2::scale(size) * Mat2x2::rotate(angle);
        frame.draw_path(&path, path_key, Vec2::new(0.0, 0.0), transform, Color::rgba(0.0, 0.0, 0.0, 1.0));
        frame.finish();

        context.swap_buffers().unwrap();

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => {
                use glutin::WindowEvent::*;
                match event {
                    CloseRequested => {
                        running = false;
                    }
                    MouseWheel { delta, .. } => match delta {
                        glutin::MouseScrollDelta::PixelDelta(position) => {
                            size *= 1.01f32.powf(-position.y as f32);
                        }
                        glutin::MouseScrollDelta::LineDelta(_dx, dy) => {
                            size *= 1.01f32.powf(dy as f32 * 12.0);
                        }
                    },
                    KeyboardInput { input, .. } => match input {
                        glutin::KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        } => match key {
                            glutin::VirtualKeyCode::A => match state {
                                glutin::ElementState::Pressed => {
                                    left = true;
                                }
                                glutin::ElementState::Released => {
                                    left = false;
                                }
                            },
                            glutin::VirtualKeyCode::D => match state {
                                glutin::ElementState::Pressed => {
                                    right = true;
                                }
                                glutin::ElementState::Released => {
                                    right = false;
                                }
                            },
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        });

        let elapsed = now.elapsed();
        if elapsed < FRAME {
            std::thread::sleep(FRAME - elapsed);
        }
        now = std::time::Instant::now();
    }
}
