use log::{info, warn};
use std::collections::HashMap;
use std::error::Error;
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;

enum SocketChange {
    Added(SocketAddr, (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>)),
    Removed(SocketAddr),
}

pub fn start(host: &str, port: u16, device: &str) -> Result<(), Box<dyn Error>> {
    // Connect to serial device
    let serial_device = serialport::new(device, 9600).open()?;
    info!("Connected to {}", device);

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
    info!("Listening on {}", addr);

    loop {
        // Get connections from socket
        let (socket, peer_addr) = listener.accept()?;
        info!("Got connection from {}", peer_addr);

        // Create channels for connection
        let (socket_tx_sender, socket_tx_receiver) = mpsc::channel::<Vec<u8>>();
        let (socket_rx_sender, socket_rx_receiver) = mpsc::channel::<Vec<u8>>();

        // Notify coordinator that a new socket connected
        coordinator_channels_sender
            .send(SocketChange::Added(
                peer_addr,
                (socket_tx_sender, socket_rx_receiver),
            ))
            .expect("Control: Unable to send channels to coordinator");

        // Move ownership of the socket and channels into the created thread
        spawn_socket_thread(
            coordinator_channels_sender.clone(),
            socket,
            socket_rx_sender,
            socket_tx_receiver,
        );
    }
}

fn spawn_socket_thread(
    channels_sender: mpsc::Sender<SocketChange>,
    socket: TcpStream,
    rx_sender: mpsc::Sender<std::vec::Vec<u8>>,
    tx_receiver: mpsc::Receiver<std::vec::Vec<u8>>,
) {
    let mut socket_rx = socket.try_clone().expect("Socket: Unable to clone socket");
    let mut socket_tx = socket.try_clone().expect("Socket: Unable to clone socket");

    let channels_sender_tx = channels_sender.clone();
    let channels_sender_rx = channels_sender.clone();

    std::thread::spawn(move || loop {
        let mut buf = vec![0; 64];
        let bytes_read = match socket_rx.read(&mut buf) {
            Ok(0) => {
                // Handle socket disconnect
                let peer_addr = socket_rx
                    .peer_addr()
                    .expect("Socket: Unable to get peer address");
                info!("Lost connection from {:?}", peer_addr);
                channels_sender_tx
                    .send(SocketChange::Removed(peer_addr))
                    .expect("Socket: Unable to send channel removal");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                warn!("Socket error: {}", e);
                0
            }
        };
        let received_data = buf[..bytes_read].to_vec();
        if let Err(e) = rx_sender.send(received_data) {
            warn!("Channel put error: {}", e);
        }
    });

    std::thread::spawn(move || loop {
        match tx_receiver.recv() {
            Ok(rx_data) => {
                socket_tx
                    .write_all(&rx_data)
                    .expect("Socket: Unable to write to socket");
            }
            Err(_) => {
                let peer_addr = socket_tx
                    .peer_addr()
                    .expect("Socket: Unable to get peer address");
                channels_sender_rx
                    .send(SocketChange::Removed(peer_addr))
                    .expect("Socket: Unable to send channel removal");
                break;
            }
        }
    });
}

fn create_coordinator_thread(
    channels_receiver: mpsc::Receiver<SocketChange>,
    serial_tx: mpsc::Sender<std::vec::Vec<u8>>,
    serial_rx: mpsc::Receiver<std::vec::Vec<u8>>,
) {
    std::thread::spawn(move || {
        // Create storage of socket channels
        let mut sockets_map: HashMap<
            SocketAddr,
            (
                mpsc::Sender<std::vec::Vec<u8>>,
                mpsc::Receiver<std::vec::Vec<u8>>,
            ),
        > = HashMap::new();

        loop {
            // Check for new channels to keep track of
            if let Ok(item) = channels_receiver.try_recv() {
                match item {
                    SocketChange::Added(addr, channels) => {
                        sockets_map.insert(addr, channels);
                    }
                    SocketChange::Removed(addr) => {
                        sockets_map.remove(&addr);
                    }
                };
            }

            // Transfer data from serial_rx_channel to multiple socket_tx_channel
            if let Ok(rx_data) = serial_rx.try_recv() {
                // For each socket
                for (socket_tx_channel, _) in sockets_map.values() {
                    // Send received data to each socket
                    socket_tx_channel
                        .send(rx_data.clone())
                        .expect("Coordinator: Unable to send to socket channel");
                }
            }
            // Transfer data from multiple socket_rx_channel to serial_tx_channel
            for (_, socket_rx_channel) in sockets_map.values() {
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
                warn!("Serial: Serial TX channel closed: {}", e);
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
