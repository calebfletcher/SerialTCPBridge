use serial_tcp_bridge;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    serial_tcp_bridge::start()
}
