use egui::Context;
use once_cell::sync::OnceCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

const ACTIVATE_ADDR: &str = "127.0.0.1:57577";

pub fn setup_single_instance(
    activate_tx: crossbeam::channel::Sender<()>,
    wake: Arc<OnceCell<Context>>,
) -> bool {
    match TcpListener::bind(ACTIVATE_ADDR) {
        Ok(listener) => {
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        let mut _buf = [0u8; 4];
                        let _ = s.read(&mut _buf);
                        let _ = activate_tx.send(());
                        if let Some(ctx) = wake.get() {
                            ctx.request_repaint();
                        }
                    }
                }
            });
            true
        }
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect(ACTIVATE_ADDR) {
                let _ = s.write_all(b"SHOW");
            }
            false
        }
    }
}
