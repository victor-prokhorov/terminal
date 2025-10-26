use eframe::egui;
use nix::{
    errno::Errno,
    fcntl::{FcntlArg, OFlag, fcntl},
    pty::ForkptyResult,
};
use std::{
    env,
    ffi::CString,
    os::fd::{AsFd, OwnedFd},
};

fn main() {
    let fd = unsafe {
        let result = nix::pty::forkpty(None, None).expect("failed to fork pty");
        match result {
            ForkptyResult::Parent { master, .. } => master,
            ForkptyResult::Child => {
                env::remove_var("ENV");
                env::set_var("PS1", "% ");
                nix::unistd::execvp::<CString>(c"ash", &[c"ash".to_owned()])
                    .expect("failed to replace current process image");
                return;
            }
        }
    };
    eframe::run_native(
        "App",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc, fd)))),
    )
    .expect("failed to start the app");
}

struct App {
    fd: OwnedFd,
    output: Vec<u8>,
    input: Vec<u8>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        let flags =
            fcntl(fd.as_fd(), FcntlArg::F_GETFL).expect("failed to get descriptor status flags");
        let mut flags = OFlag::from_bits(flags & OFlag::O_ACCMODE.bits())
            .expect("failed to create configuration options for opened file");
        flags.set(OFlag::O_NONBLOCK, true);
        fcntl(fd.as_fd(), FcntlArg::F_SETFL(flags)).expect("failed to set descriptor status flags");
        App {
            fd,
            output: Vec::new(),
            input: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        assert!(self.output.len() < 1_000_000);
        let mut buf = vec![0; 4096];
        match nix::unistd::read(self.fd.as_fd(), &mut buf) {
            Err(Errno::EAGAIN) => (),
            Err(e) => eprintln!("failed to read: {e}"),
            Ok(read) => self.output.extend_from_slice(&buf[0..read]),
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.input(|input_state| {
                for event in &input_state.events {
                    match event {
                        egui::Event::Text(text) => {
                            self.input.extend_from_slice(text.as_bytes());
                            assert!(self.input.len() < 4096);
                        }
                        egui::Event::Key {
                            key: egui::Key::Enter,
                            pressed: true,
                            ..
                        } => {
                            self.input.push(b'\n');
                            nix::unistd::write(self.fd.as_fd(), self.input.as_slice())
                                .expect("failed to write to file descriptor");
                            self.input.clear();
                        }
                        _ => (),
                    }
                }
            });
            unsafe {
                ui.label(format!(
                    "{}{}",
                    std::str::from_utf8_unchecked(&self.output),
                    std::str::from_utf8_unchecked(&self.input)
                ));
            }
        });
    }
}
