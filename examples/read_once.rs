extern crate socketcan;
extern crate log;
extern crate env_logger;

use socketcan::socket::CanSocket;
use std::time::UNIX_EPOCH;

fn main() {
    env_logger::init();    
    let bus  = CanSocket::open("vcan0").unwrap();

    loop {
        match bus.read() {
            Ok((frame, time)) => log::debug!("[{:?}] {:?}", time.duration_since(UNIX_EPOCH).unwrap().as_secs(), frame),
            Err(e) => { log::debug!("Error: {}", e); break; },
        }
    }
}