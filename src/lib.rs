use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    stream.write_all(b"hello world")?;
    stream.shutdown(Shutdown::Both)
}

pub fn start(host: &str, port: u16, device: &str, verbose: i32) -> Result<(), Box<dyn Error>> {
    // Connect to serial device
    let serial_device = serialport::new(device, 9600).open()?;
    //let serial_reader = std::io::BufReader::new(serial_device);
    println!("Connected to {}", device);

    //  One serial port, multiple TCP sockets
    //  When data is received from serial, it is sent to all TCP sockets
    //      Needs a channel per socket
    //      Sender side of channel goes to serial port
    //      Receiver side of channel goes to TCP socket
    //  When data is received from any TCP socket, it is sent to the serial port
    //      Needs a channel per socket
    //      Sender side of channel goes to TCP socket
    //
    //  Threads:
    //      Socket Control Thread
    //          Listen for new connections
    //          Create channels for connections
    //          Handle disconnection of connections
    //      Coordinator Thread
    //          Checks for new channels to keep track of
    //          Transfer data from serial_rx_channel to multiple socket_tx_channel
    //          Transfer data from multiple socket_rx_channel to serial_tx_channel
    //      Socket Rx Thread - one per socket
    //          Listen for data incoming from socket
    //          Put incoming data into socket_rx_channel
    //      Socket Tx Thread - one per socket
    //          Check socket_tx_channel for new data to be transmitted
    //          Transmit new data
    //      Serial Rx Thread
    //          Listen for data incoming from serial port
    //          Put data into serial_rx_channel
    //      Serial Tx Thread
    //          Check serial_tx_channel for new items
    //          Transmit new data over the serial port
    //

    // Set up serial channels
    let (serial_tx_sender, serial_tx_receiver) = mpsc::channel::<Vec<u8>>();
    let (serial_rx_sender, serial_rx_receiver) = mpsc::channel::<Vec<u8>>();

    create_serial_threads(serial_device, serial_tx_receiver, serial_rx_sender)
        .expect("Unable to create serial port threads");

    // Set up broadcasting channels
    let mut serial_to_socket_channels: Vec<mpsc::Sender<Vec<u8>>> = Vec::new();

    // Create TCP server
    let ip: IpAddr = host.parse()?;
    let addr = SocketAddr::new(ip, port);
    let listener = TcpListener::bind(addr)?;
    println!("Listening on {}", addr);

    loop {
        // Get connections from socket
        let (socket, addr) = listener.accept()?;
        println!("Got connection from {}", addr);

        // Create channels for connection
        let (sender, receiver_channel) = mpsc::channel::<Vec<u8>>();
        serial_to_socket_channels.push(sender);
        //let sender_channel = socket_to_serial_sender.clone();

        // Move ownership of the socket and channels into the created thread
        //spawn_socket_thread(socket, sender_channel, receiver_channel);
    }
}

fn spawn_socket_thread(
    mut socket: TcpStream,
    sender: mpsc::Sender<std::vec::Vec<u8>>,
    receiver: mpsc::Receiver<std::vec::Vec<u8>>,
) {
    std::thread::spawn(move || loop {
        let mut buf = vec![0; 64];
        let bytes_read = match socket.read(&mut buf) {
            Ok(0) => {
                eprintln!("Socket closed or empty");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                eprintln!("Socker error: {}", e);
                0
            }
        };
        let received_data = buf[..bytes_read].to_vec();
        if let Err(e) = sender.send(received_data) {
            eprintln!("Channel put error: {}", e);
        }
    });
}

fn create_serial_threads(
    device: Box<dyn serialport::SerialPort>,
    tx_channel: mpsc::Receiver<std::vec::Vec<u8>>,
    rx_channel: mpsc::Sender<std::vec::Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let mut serial_device_tx = device.try_clone()?;
    let mut serial_device_rx = device.try_clone()?;

    // Serial Tx Thread
    std::thread::spawn(move || loop {
        let mut item = match tx_channel.recv() {
            Err(e) => {
                eprintln!("Serial TX channel closed: {}", e);
                break;
            }
            Ok(item) => item,
        };
        serial_device_tx
            .write_all(&mut item)
            .expect("Unable to write to serial port");
    });
    // Serial Rx Thread
    std::thread::spawn(move || loop {
        let mut buf = vec![0; 128];
        let bytes_read = serial_device_rx
            .read(&mut buf)
            .expect("Unable to read from serial port");
        rx_channel
            .send(buf[..bytes_read].to_vec())
            .expect("Unable to send data from serial port to channel")
    });

    Ok(())
}
