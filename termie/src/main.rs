use eframe::egui;
use nix::{
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
    fd: OwnedFd,
    buf: Vec<u8>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        let flags =
            fcntl(fd.as_fd(), FcntlArg::F_GETFL).expect("failed to get descriptor satus flags");
        let mut flags = OFlag::from_bits(flags)
            .expect("failed to create configuration options for opened file");
        flags.set(OFlag::O_NONBLOCK, true);
        fcntl(fd.as_fd(), FcntlArg::F_SETFL(flags)).expect("failed to set descriptor status flags");
        App {
            fd,
            buf: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut buf = vec![0; 4096];
        match self.fd.read(&mut buf) {
            Err(e) => eprintln!("failed to read: {e}"),
            Ok(read) => self.buf.extend_from_slice(&buf[0..read]),
        }
        egui::CentralPanel::default().show(ctx, |ui| unsafe {
            ui.label(std::str::from_utf8_unchecked(&self.buf))
        });
    }
}
