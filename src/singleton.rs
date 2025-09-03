use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const ACTIVATE_ADDR: &str = "127.0.0.1:57577";

pub fn setup_single_instance(activate_tx: crossbeam::channel::Sender<()>) -> bool {
    match TcpListener::bind(ACTIVATE_ADDR) {
        Ok(listener) => {
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        let mut _buf = [0u8; 4];
                        let _ = s.read(&mut _buf);
                        let _ = activate_tx.send(());
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
