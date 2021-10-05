# SerialTCPBridge
A Rust implementation of a scalable serial port to TCP bridge, supporting multiple concurrent TCP connections.

## Architecture
One serial port, multiple TCP sockets
When data is received from serial, it is sent to all TCP sockets
1. Needs a channel per socket
2. Sender side of channel goes to serial port
3. Receiver side of channel goes to TCP socket
When data is received from any TCP socket, it is sent to the serial port
1. Needs a channel per socket
2. Sender side of channel goes to TCP socket

### Threads
#### Socket Control Thread
Listen for new connections
Create channels for connections
Handle disconnection of connections
#### Coordinator Thread
Checks for new channels to keep track of
Transfer data from serial_rx_channel to multiple socket_tx_channel
Transfer data from multiple socket_rx_channel to serial_tx_channel
#### Socket Rx Thread - one per socket
Listen for data incoming from socket
Put incoming data into socket_rx_channel
#### Socket Tx Thread - one per socket
Check socket_tx_channel for new data to be transmitted
Transmit new data
#### Serial Rx Thread
Listen for data incoming from serial port
Put data into serial_rx_channel
#### Serial Tx Thread
Check serial_tx_channel for new items
Transmit new data over the serial port