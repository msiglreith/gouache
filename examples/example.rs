use gouache::{Color, Graphics, PathBuilder};

const FRAME: std::time::Duration = std::time::Duration::from_micros(1_000_000 / 60);

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 600.0))
        .with_title("gouache");
    let context = glutin::ContextBuilder::new()
        .build_windowed(window_builder, &events_loop).unwrap();
    let context = unsafe { context.make_current() }.unwrap();

    gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

    let mut graphics = Graphics::new(800.0, 600.0);

    let font = graphics.add_font(include_bytes!("../res/SourceSansPro-Regular.ttf")).unwrap();

    let path = PathBuilder::new()
        .move_to(0.5, 1.0)
        .line_to(1.0, 0.5)
        .line_to(0.5, 0.0)
        .line_to(0.0, 0.5)
        .build(&mut graphics);

    let mut size = 14.0;

    let mut running = true;
    let mut now = std::time::Instant::now();
    while running {
        graphics.clear(Color::rgba(0.1, 0.15, 0.2, 1.0));
        graphics.begin_frame();
        graphics.draw_text(0.0, 0.0, size, font, Color::rgba(1.0, 1.0, 1.0, 1.0), "jackdaws love my big sphinx of quartz 1234567890");
        graphics.draw_path(100.0, 100.0, 50.0, Color::rgba(0.0, 1.0, 1.0, 1.0), path);
        graphics.end_frame();

        context.swap_buffers().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => {
                    use glutin::WindowEvent::*;
                    match event {
                        CloseRequested => {
                            running = false;
                        }
                        MouseWheel { delta, .. } => {
                            match delta {
                                glutin::MouseScrollDelta::PixelDelta(position) => { size -= 0.1 * position.y as f32; }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        });

        let elapsed = now.elapsed();
        if elapsed < FRAME {
            std::thread::sleep(FRAME - elapsed);
        }
        now = std::time::Instant::now();
    }
}
