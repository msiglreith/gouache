use crate::graphics::{Color, Graphics, Point};

const FRAME: std::time::Duration = std::time::Duration::from_micros(1_000_000 / 60);

pub struct Window {
    events_loop: glutin::EventsLoop,
    context: glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::Window>,
    graphics: Graphics,
}

impl Window {
    pub fn new() -> Window {
        let mut events_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 600.0))
            .with_title("gouache");
        let context = glutin::ContextBuilder::new()
            .build_windowed(window_builder, &events_loop).unwrap();
        let context = unsafe { context.make_current() }.unwrap();

        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

        let graphics = Graphics::new(800.0, 600.0);

        Window { events_loop, context, graphics }
    }

    pub fn run(&mut self) {
        let mut running = true;
        let mut now = std::time::Instant::now();
        let mut fps_counter = FpsCounter::new();
        while running {
            let elapsed = now.elapsed();
            now = std::time::Instant::now();
            fps_counter.update(elapsed);

            self.graphics.clear(Color::rgba(0.3, 0.56, 0.7, 1.0));
            self.graphics.begin_frame();
            self.graphics.set_color(Color::rgba(1.0, 1.0, 1.0, 1.0));
            self.graphics.draw_path();
            self.graphics.end_frame();

            self.context.swap_buffers().unwrap();

            self.events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, .. } => {
                        use glutin::WindowEvent::*;
                        match event {
                            Resized(size) => {}
                            Moved(position) => {}
                            CloseRequested => {
                                running = false;
                            }
                            Destroyed => {}
                            DroppedFile(path) => {}
                            HoveredFile(path) => {}
                            HoveredFileCancelled => {}
                            ReceivedCharacter(c) => {}
                            Focused(focus) => {}
                            KeyboardInput { input, .. } => {}
                            CursorMoved { position, modifiers, .. } => {}
                            CursorEntered { .. } => {}
                            CursorLeft { .. } => {}
                            MouseWheel { delta, modifiers, .. } => {}
                            MouseInput { state, button, modifiers, .. } => {}
                            Refresh => {}
                            HiDpiFactorChanged(factor) => {}
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
        }
    }
}

struct FpsCounter {
    frames: [u32; 100],
    i: usize,
    sum: u32,
}

impl FpsCounter {
    fn new() -> FpsCounter {
        FpsCounter {
            frames: [0; 100],
            i: 0,
            sum: 0,
        }
    }

    fn update(&mut self, elapsed: std::time::Duration) {
        self.sum -= self.frames[self.i];
        self.frames[self.i] = elapsed.as_secs() as u32 * 1000000 + elapsed.subsec_micros();
        self.sum += self.frames[self.i];
        self.i = (self.i + 1) % self.frames.len();
    }

    fn fps(&self) -> f32 {
        100000000.0 / (self.sum as f32)
    }
}
