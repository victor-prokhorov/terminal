mod http_client;

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
enum Message {
    Classification { input: Vec<u8>, is_command: bool },
    Error { msg: String },
}

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
    runtime: tokio::runtime::Runtime,
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        let flags =
            fcntl(fd.as_fd(), FcntlArg::F_GETFL).expect("failed to get descriptor status flags");
        let mut flags = OFlag::from_bits(flags & OFlag::O_ACCMODE.bits())
            .expect("failed to create configuration options for opened file");
        flags.set(OFlag::O_NONBLOCK, true);
        fcntl(fd.as_fd(), FcntlArg::F_SETFL(flags)).expect("failed to set descriptor status flags");
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        App {
            fd,
            output: Vec::new(),
            input: Vec::new(),
            runtime,
            tx,
            rx,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        assert!(self.output.len() < 1_000_000);
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::Classification { input, is_command } => {
                    if is_command {
                        nix::unistd::write(self.fd.as_fd(), &input)
                            .expect("failed to write to file descriptor");
                    } else {
                        self.output.extend_from_slice(&input);
                        self.output.extend_from_slice(b"natural language query received");
                        nix::unistd::write(self.fd.as_fd(), b"\n")
                            .expect("failed to write to file descriptor");
                    }
                    ctx.request_repaint();
                }
                Message::Error { msg } => {
                    eprintln!("error: {msg}");
                }
            }
        }
        let mut buf = vec![0; 4096];
        match nix::unistd::read(self.fd.as_fd(), &mut buf) {
            Err(Errno::EAGAIN) => (),
            Err(e) => eprintln!("failed to read: {e}"),
            Ok(read) => {
                self.output.extend_from_slice(&buf[0..read]);
                ctx.request_repaint();
            }
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
                            if self.input.len() == 1 {
                                self.input.clear();
                                nix::unistd::write(self.fd.as_fd(), b"\n")
                                    .expect("failed to write to file descriptor");
                                continue;
                            }
                            let input = String::from_utf8_lossy(&self.input).to_string();
                            let tx = self.tx.clone();
                            let ctx = ctx.clone();
                            self.runtime.spawn(async move {
                                match http_client::classify(&input).await {
                                    Ok(is_command) => {
                                        tx.send(Message::Classification {
                                            input: input.as_bytes().to_vec(),
                                            is_command,
                                        })
                                        .expect("failed to send");
                                    }
                                    Err(e) => {
                                        tx.send(Message::Error {
                                            msg: format!("classification failed: {e}"),
                                        })
                                        .expect("failed to send");
                                    }
                                }
                                ctx.request_repaint();
                            });
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
