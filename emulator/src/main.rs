use fontdue::{
    Font,
    layout::{CoordinateSystem, Layout, TextStyle},
};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const DEJA_VU_SANS_MONO: &[u8] =
    include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    font: Font,
    layout: Layout,
    input: String,
    output: String,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(win) =
            event_loop.create_window(Window::default_attributes().with_title("terminal"))
        {
            let window = Rc::new(win);
            if let Ok(context) = Context::new(window.clone())
                && let Ok(surface) = Surface::new(&context, window.clone())
            {
                self.window = Some(window);
                self.surface = Some(surface);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    match event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            self.input.pop();
                        }
                        Key::Named(NamedKey::Enter) => {
                            if !self.input.is_empty() {
                                use std::process::Command;
                                let parts: Vec<&str> = self.input.split_whitespace().collect();
                                if let Some(cmd) = parts.first() {
                                    let args = &parts[1..];
                                    match Command::new(cmd).args(args).output() {
                                        Ok(output) => {
                                            self.output = format!(
                                                "{}{}",
                                                String::from_utf8_lossy(&output.stdout),
                                                String::from_utf8_lossy(&output.stderr)
                                            );
                                        }
                                        Err(e) => {
                                            self.output = e.to_string();
                                        }
                                    }
                                }
                                self.input.clear();
                            }
                        }
                        Key::Named(NamedKey::Space) => {
                            self.input.push(' ');
                        }
                        Key::Character(c) => {
                            self.input.push_str(c.as_str());
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    let inner_size = window.inner_size();
                    if let (Some(width), Some(height)) = (
                        NonZeroU32::new(inner_size.width),
                        NonZeroU32::new(inner_size.height),
                    ) {
                        surface.resize(width, height).expect("failed to resize");
                        if let Ok(mut buffer) = surface.buffer_mut() {
                            for pixel in buffer.iter_mut() {
                                *pixel = 0x00_00_00;
                            }
                            let display = if self.input.is_empty() {
                                &self.output
                            } else {
                                &self.input
                            };
                            let font_size = 16.0;
                            let line_height = 20;
                            let xp = 10;
                            let mut yp = 10;
                            for line in display.lines() {
                                self.layout.clear();
                                self.layout
                                    .append(&[&self.font], &TextStyle::new(line, font_size, 0));
                                for glyph in self.layout.glyphs() {
                                    let (metrics, bitmap) =
                                        self.font.rasterize(glyph.parent, glyph.key.px);
                                    for my in 0..metrics.height {
                                        for mx in 0..metrics.width {
                                            let x = xp + glyph.x as usize + mx;
                                            let y = yp + glyph.y as usize + my;
                                            if x < width.get() as usize && y < height.get() as usize
                                            {
                                                let alpha = bitmap[my * metrics.width + mx];
                                                buffer[y * width.get() as usize + x] =
                                                    u32::from(alpha) * 0x01_01_01;
                                            }
                                        }
                                    }
                                }
                                yp += line_height;
                            }
                            buffer
                                .present()
                                .expect("failed to presetn buffer to the window");
                        }
                    }
                }
            }
            _ => {
                eprintln!("unhandled window event {event:?}");
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    if let Ok(event_loop) = EventLoop::new() {
        event_loop.set_control_flow(ControlFlow::Wait);
        let font = Font::from_bytes(DEJA_VU_SANS_MONO, fontdue::FontSettings::default())
            .expect("failed to load font");
        let layout = Layout::new(CoordinateSystem::PositiveYDown);
        let mut app = App {
            window: None,
            surface: None,
            font,
            layout,
            input: String::new(),
            output: String::new(),
        };
        event_loop
            .run_app(&mut app)
            .expect("event loop failed to run the app");
    }
}
