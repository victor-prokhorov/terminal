use eframe::egui;
use nix::pty::ForkptyResult;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::Read,
    os::fd::OwnedFd,
};

fn main() {
    let fd = unsafe {
        let result = nix::pty::forkpty(None, None).expect("pty fork failed");
        match result {
            ForkptyResult::Parent { master, .. } => master,
            ForkptyResult::Child => {
                let shell = CStr::from_bytes_with_nul(b"bash\0").expect("nul termination missing");
                assert_eq!(shell, c"bash"); // TODO: use CStr directly
                nix::unistd::execvp::<CString>(shell, &[])
                    .expect("failed to replace current process image");
                return; // TODO: find better way
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
