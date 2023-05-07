use std::{error::Error, net::SocketAddr};

use crate::{ErrorCode, Packet, Socket, TransferOption};

/// Message `struct` is used for easy message transmission of common TFTP
/// message types.
///
/// # Example
///
/// ```rust
/// use std::{net::{SocketAddr, UdpSocket}, str::FromStr};
/// use tftpd::{Message, ErrorCode};
///
/// // Send a FileNotFound error.
/// Message::send_error_to(
///     &UdpSocket::bind(SocketAddr::from_str("127.0.0.1:6969").unwrap()).unwrap(),
///     &SocketAddr::from_str("127.0.0.1:1234").unwrap(),
///     ErrorCode::FileNotFound,
///     "file does not exist",
/// );
/// ```
pub struct Message;

const MAX_REQUEST_PACKET_SIZE: usize = 512;

impl Message {
    /// Sends a data packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_data<T: Socket>(
        socket: &T,
        block_num: u16,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Data { block_num, data })
    }

    /// Sends an acknowledgement packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_ack<T: Socket>(socket: &T, block_number: u16) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Ack(block_number))
    }

    /// Sends an error packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_error<T: Socket>(
        socket: &T,
        code: ErrorCode,
        msg: &str,
    ) -> Result<(), Box<dyn Error>> {
        if socket
            .send(&Packet::Error {
                code,
                msg: msg.to_string(),
            })
            .is_err()
        {
            eprintln!("could not send an error message");
        };

        Err(msg.into())
    }

    /// Sends an error packet to the supplied [`SocketAddr`].
    pub fn send_error_to<T: Socket>(
        socket: &T,
        to: &SocketAddr,
        code: ErrorCode,
        msg: &str,
    ) -> Result<(), Box<dyn Error>> {
        if socket
            .send_to(
                &Packet::Error {
                    code,
                    msg: msg.to_string(),
                },
                to,
            )
            .is_err()
        {
            eprintln!("could not send an error message");
        }
        Err(msg.into())
    }

    /// Sends an option acknowledgement packet to the socket's connected remote.
    /// See [`UdpSocket`] for more information about connected sockets.
    pub fn send_oack<T: Socket>(
        socket: &T,
        options: Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Oack(options))
    }

    /// Receives a packet from the socket's connected remote, and returns the
    /// parsed [`Packet`]. This function cannot handle large data packets due to
    /// the limited buffer size. For handling data packets, see [`Message::recv_with_size()`].
    pub fn recv<T: Socket>(socket: &T) -> Result<Packet, Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        socket.recv(&mut buf)
    }

    /// Receives a packet from any incoming remote request, and returns the
    /// parsed [`Packet`] and the requesting [`SocketAddr`]. This function cannot handle
    /// large data packets due to the limited buffer size, so it is intended for
    /// only accepting incoming requests. For handling data packets, see [`Message::recv_with_size()`].
    pub fn recv_from<T: Socket>(socket: &T) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        socket.recv_from(&mut buf)
    }

    /// Receives a data packet from the socket's connected remote, and returns the
    /// parsed [`Packet`]. The received packet can actually be of any type, however,
    /// this function also allows supplying the buffer size for an incoming request.
    pub fn recv_with_size<T: Socket>(socket: &T, size: usize) -> Result<Packet, Box<dyn Error>> {
        let mut buf = vec![0; size + 4];
        socket.recv(&mut buf)
    }

    /// Receives a data packet from any incoming remote request, and returns the
    /// parsed [`Packet`] and the requesting [`SocketAddr`]. The received packet can
    /// actually be of any type, however, this function also allows supplying the
    /// buffer size for an incoming request.
    pub fn recv_from_with_size<T: Socket>(
        socket: &T,
        size: usize,
    ) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        let mut buf = vec![0; size + 4];
        socket.recv_from(&mut buf)
    }
}
