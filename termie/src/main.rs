use eframe::egui;
use nix::pty::ForkptyResult;
use std::{env, ffi::CString, fs::File, io::Read, os::fd::OwnedFd};

fn main() {
    let fd = unsafe {
        let result = nix::pty::forkpty(None, None).expect("pty fork failed");
        match result {
            ForkptyResult::Parent { master, .. } => master,
            ForkptyResult::Child => {
                let args = [c"ash"];
                let args: Vec<CString> = args.into_iter().map(ToOwned::to_owned).collect();
                env::remove_var("PROMPT_COMMAND");
                env::remove_var("ENV");
                env::set_var("PS1", "% ");
                nix::unistd::execvp::<CString>(c"ash", &args)
                    .expect("failed to replace current process image");
                return;
            }
        }
    };
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "App",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, fd)))),
    );
}

struct App {
    file: File,
    buf: Vec<u8>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        App {
            file: fd.into(),
            buf: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut buf = vec![0; 4096];
        match self.file.read(&mut buf) {
            Err(e) => eprintln!("failed to read: {e}"),
            Ok(read) => self.buf.extend_from_slice(&buf[0..read]),
        }
        egui::CentralPanel::default().show(ctx, |ui| unsafe {
            ui.label(std::str::from_utf8_unchecked(&self.buf))
        });
    }
}
