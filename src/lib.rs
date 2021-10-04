use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    stream.write_all(b"hello world")?;
    stream.shutdown(Shutdown::Both)
}

pub fn start(host: &str, port: u16, device: &str, verbose: i32) -> Result<(), Box<dyn Error>> {
    let mut serial_device = serialport::new(device, 9600).open()?;
    //let mut write_device = serial_device.try_clone()?;
    //let mut serial_reader = std::io::BufReader::new(&mut serial_device);
    println!("Connected to {}", device);

    // Write to port
    //write_device.write_all(b"hello world")?;

    let output = "This is a test.\r\n".as_bytes();
    serial_device.write(output).expect("Write failed!");
    serial_device.flush().unwrap();

    let mut reader = std::io::BufReader::new(serial_device);

    loop {
        let mut my_str = String::new();
        loop {
            match reader.read_line(&mut my_str) {
                Ok(_) => break,
                Err(_) => continue,
            };
        }

        print!("{}", my_str);
    }

    let ip: IpAddr = host.parse()?;
    let addr = SocketAddr::new(ip, port);
    println!("Listening on {}", addr);
    let listener = TcpListener::bind(addr)?;

    loop {
        let (socket, addr) = listener.accept()?;
        if verbose >= 1 {
            println!("Got connection from {}", addr);
        }
        handle_client(socket)?;
    }
}
