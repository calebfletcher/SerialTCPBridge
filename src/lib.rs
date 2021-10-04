use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    stream.write_all(b"hello world")?;
    stream.shutdown(Shutdown::Both)
}

pub fn start(host: &str, port: u16, verbose: i32) -> Result<(), Box<dyn Error>> {
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
