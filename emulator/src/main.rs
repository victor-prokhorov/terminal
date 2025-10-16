use fontdue::{
    Font,
    layout::{CoordinateSystem, Layout, TextStyle},
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use softbuffer::{Context, Surface};
use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
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
    pty_writer: Box<dyn Write + Send>,
    pty_output: Receiver<String>,
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
                                writeln!(self.pty_writer, "{}", self.input).expect("failed to write");
                                self.pty_writer.flush().expect("failed to flush");
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
                            let clean_output = strip_ansi_codes(&self.output);
                            let display = if self.input.is_empty() {
                                clean_output.as_str()
                            } else {
                                &format!("{}{}", clean_output, self.input)
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
        while let Ok(output) = self.pty_output.try_recv() {
            self.output.push_str(&output);
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let font = Font::from_bytes(DEJA_VU_SANS_MONO, fontdue::FontSettings::default())
        .expect("failed to load font");
    let layout = Layout::new(CoordinateSystem::PositiveYDown);
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let cmd = CommandBuilder::new("bash");
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader()?;
    let pty_writer = pair.master.take_writer()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    break;
                }
                Ok(bytes_read) => {
                    let output = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                    if tx.send(output).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    break;
                }
            }
        }
        let status = child.wait().expect("failed to wait child");
        println!("status {status}");
    });
    let mut app = App {
        window: None,
        surface: None,
        font,
        layout,
        input: String::new(),
        output: String::new(),
        pty_writer,
        pty_output: rx,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    if next_ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                chars.next();
                while let Some(next_ch) = chars.next() {
                    if next_ch == '\x07' || (next_ch == '\x1b' && chars.peek() == Some(&'\\')) {
                        if next_ch == '\x1b' {
                            chars.next();
                        }
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}
