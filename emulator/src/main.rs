use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
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
        let mut app = App {
            window: None,
            surface: None,
        };
        event_loop
            .run_app(&mut app)
            .expect("event loop failed to run the app");
    }
}
