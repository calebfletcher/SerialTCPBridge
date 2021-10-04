use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;

pub fn start(host: &str, port: u16, device: &str, _verbose: i32) -> Result<(), Box<dyn Error>> {
    // Connect to serial device
    let serial_device = serialport::new(device, 9600).open()?;
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

    // Set up coordinator channel
    let (coordinator_channels_sender, coordinator_channels_receiver) = mpsc::channel();

    // Set up serial channels
    let (serial_tx_sender, serial_tx_receiver) = mpsc::channel();
    let (serial_rx_sender, serial_rx_receiver) = mpsc::channel();

    create_serial_threads(serial_device, serial_tx_receiver, serial_rx_sender)
        .expect("Unable to create serial port threads");
    create_coordinator_thread(
        coordinator_channels_receiver,
        serial_tx_sender,
        serial_rx_receiver,
    );

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
        let (socket_tx_sender, socket_tx_receiver) = mpsc::channel::<Vec<u8>>();
        let (socket_rx_sender, socket_rx_receiver) = mpsc::channel::<Vec<u8>>();

        coordinator_channels_sender
            .send((socket_tx_sender, socket_rx_receiver))
            .expect("Control: Unable to send channels to coordinator");

        // Move ownership of the socket and channels into the created thread
        spawn_socket_thread(socket, socket_rx_sender, socket_tx_receiver);
    }
}

fn spawn_socket_thread(
    socket: TcpStream,
    rx_sender: mpsc::Sender<std::vec::Vec<u8>>,
    tx_receiver: mpsc::Receiver<std::vec::Vec<u8>>,
) {
    let mut socket_rx = socket.try_clone().expect("Socket: Unable to clone socket");
    let mut socket_tx = socket.try_clone().expect("Socket: Unable to clone socket");
    std::thread::spawn(move || loop {
        let mut buf = vec![0; 64];
        let bytes_read = match socket_rx.read(&mut buf) {
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
        if let Err(e) = rx_sender.send(received_data) {
            eprintln!("Channel put error: {}", e);
        }
    });

    std::thread::spawn(move || loop {
        let rx_data = tx_receiver
            .recv()
            .expect("Socket: Unable to read from tx channel");
        socket_tx
            .write_all(&rx_data)
            .expect("Socket: Unable to write to socket");
    });
}

fn create_coordinator_thread(
    channels_receiver: mpsc::Receiver<(
        mpsc::Sender<std::vec::Vec<u8>>,
        mpsc::Receiver<std::vec::Vec<u8>>,
    )>,
    serial_tx: mpsc::Sender<std::vec::Vec<u8>>,
    serial_rx: mpsc::Receiver<std::vec::Vec<u8>>,
) {
    std::thread::spawn(move || {
        let mut socket_tx_channels = Vec::new();
        let mut socket_rx_channels = Vec::new();

        loop {
            // Check for new channels to keep track of
            if let Ok(channels) = channels_receiver.try_recv() {
                let (tx_channel, rx_channel) = channels;
                socket_tx_channels.push(tx_channel);
                socket_rx_channels.push(rx_channel);
            }

            // Transfer data from serial_rx_channel to multiple socket_tx_channel
            if let Ok(rx_data) = serial_rx.try_recv() {
                // For each socket
                for socket_tx_channel in &socket_tx_channels {
                    // Send received data to each socket
                    socket_tx_channel
                        .send(rx_data.clone())
                        .expect("Coordinator: Unable to send to socket channel");
                }
            }
            // Transfer data from multiple socket_rx_channel to serial_tx_channel
            for socket_rx_channel in &socket_rx_channels {
                if let Ok(rx_data) = socket_rx_channel.try_recv() {
                    serial_tx
                        .send(rx_data)
                        .expect("Coordinator: Unable to send to serial channel");
                }
            }
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
                eprintln!("Serial: Serial TX channel closed: {}", e);
                break;
            }
            Ok(item) => item,
        };
        serial_device_tx
            .write_all(&mut item)
            .expect("Serial: Unable to write to serial port");
    });
    // Serial Rx Thread
    std::thread::spawn(move || loop {
        let mut buf = vec![0; 128];
        let bytes_read = match serial_device_rx.read(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let rx_data = buf[..bytes_read].to_vec();
        rx_channel
            .send(rx_data)
            .expect("Serial: Unable to send data from serial port to channel")
    });

    Ok(())
}
