extern crate socketcan;
extern crate log;

use socketcan::{CanSocket, CanFrame};

fn main() {
    
    let bus  = CanSocket::open("vcan0").unwrap();

    let data: [u8; 4] = [222, 173, 190, 239];
    let id: u32 = 123;
    let frame = CanFrame::new(id, &data, false, false).unwrap();
    match bus.write(&frame) {
        Ok(()) => log::debug!("Frame Send Success"),
        Err(e) => log::debug!("Frame Send Error {}", e),
    }
}