use gouache::{Color, Vec2, Mat2x2, PathBuilder, Frame, Font, Cache, renderers::GlRenderer};

const FRAME: std::time::Duration = std::time::Duration::from_micros(1_000_000 / 60);

const TEXT: &'static str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor
incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu
fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in
culpa qui officia deserunt mollit anim id est laborum.

Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius,
turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis
sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus
et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut
ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt
sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum.
Pellentesque malesuada nulla a mi. Duis sapien sem, aliquet nec, commodo eget,
consequat quis, neque. Aliquam faucibus, elit ut dictum aliquet, felis nisl
adipiscing sapien, sed malesuada diam lacus eget erat. Cras mollis scelerisque
nunc. Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis,
laoreet et, pretium ac, nisi. Aenean magna nisl, mollis quis, molestie eu,
feugiat in, orci. In hac habitasse platea dictumst.";

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 600.0))
        .with_title("gouache");
    let context = glutin::ContextBuilder::new()
        .build_windowed(window_builder, &events_loop)
        .unwrap();
    let context = unsafe { context.make_current() }.unwrap();

    gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

    let mut cache = Cache::new();
    let mut renderer = GlRenderer::new();

    let mut font = Font::from_bytes(include_bytes!("../res/SourceSansPro-Regular.ttf")).unwrap();
    let font_key = cache.add_font();

    let path = PathBuilder::new()
        .move_to(0.5, 1.0)
        .line_to(1.0, 0.5)
        .line_to(0.5, 0.0)
        .line_to(0.0, 0.5)
        .build();
    let path_key = cache.add_path();

    let text = font.layout("jackdaws love my big sphinx of quartz 1234567890", 14.0);

    let mut size = 1.0;
    let (mut left, mut right) = (false, false);
    let mut angle = 0.0;

    let center = 0.5 * Vec2::new(800.0, 600.0);
    let (width, height) = font.measure(TEXT, 14.0);
    let text_center = 0.5 * Vec2::new(width, height);

    let mut running = true;
    let mut now = std::time::Instant::now();
    while running {
        let mut frame = Frame::new(&mut cache, &mut renderer, 800.0, 600.0);

        frame.clear(Color::rgba(0.784, 0.804, 0.824, 1.0));

        if left {
            angle += 0.05;
        } else if right {
            angle -= 0.05;
        }

        let transform = Mat2x2::scale(size) * Mat2x2::rotate(angle);
        frame.draw_text(&font, font_key, 14.0, TEXT, center - transform * text_center, transform, Color::rgba(0.1, 0.05, 0.1, 1.0));
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
