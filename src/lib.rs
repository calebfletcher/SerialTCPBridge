use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    stream.write_all(b"hello world")?;
    stream.shutdown(Shutdown::Both)
}

pub fn start() -> Result<(), Box<dyn Error>> {
    let host = "127.0.0.1";
    let port: u16 = "41843".parse()?;
    let ip: IpAddr = host.parse().unwrap();
    let addr = SocketAddr::new(ip, port);
    println!("Listening on {}", addr);
    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        handle_client(stream?)?
    }
    Ok(())
}
