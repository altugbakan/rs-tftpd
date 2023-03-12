use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

use crate::{ErrorCode, Packet, TransferOption};

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
///     &UdpSocket::bind(SocketAddr::from_str("127.0.0.1:69").unwrap()).unwrap(),
///     &SocketAddr::from_str("127.0.0.1:1234").unwrap(),
///     ErrorCode::FileNotFound,
///     "file does not exist".to_string(),
/// );
/// ```
pub struct Message;

const MAX_REQUEST_PACKET_SIZE: usize = 512;

impl Message {
    /// Sends a data packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_data(
        socket: &UdpSocket,
        block_num: u16,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Data { block_num, data }.serialize()?)?;

        Ok(())
    }

    /// Sends an acknowledgement packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_ack(socket: &UdpSocket, block_number: u16) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Ack(block_number).serialize()?)?;

        Ok(())
    }

    /// Sends an error packet to the socket's connected remote. See
    /// [`UdpSocket`] for more information about connected
    /// sockets.
    pub fn send_error(
        socket: &UdpSocket,
        code: ErrorCode,
        msg: String,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Error { code, msg }.serialize()?)?;

        Ok(())
    }

    /// Sends an error packet to the supplied [`SocketAddr`].
    pub fn send_error_to<'a>(
        socket: &UdpSocket,
        to: &SocketAddr,
        code: ErrorCode,
        msg: String,
    ) -> Result<(), Box<dyn Error>> {
        if socket
            .send_to(
                &Packet::Error {
                    code,
                    msg: msg.clone(),
                }
                .serialize()?,
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
    pub fn send_oack(
        socket: &UdpSocket,
        options: Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Oack(options).serialize()?)?;

        Ok(())
    }

    /// Receives a packet from the socket's connected remote, and returns the
    /// parsed [`Packet`]. This function cannot handle large data packets due to
    /// the limited buffer size. For handling data packets, see [`Message::recv_data()`].
    pub fn recv(socket: &UdpSocket) -> Result<Packet, Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        let number_of_bytes = socket.recv(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok(packet)
    }

    /// Receives a packet from any incoming remote request, and returns the
    /// parsed [`Packet`] and the requesting [`SocketAddr`]. This function cannot handle
    /// large data packets due to the limited buffer size, so it is intended for
    /// only accepting incoming requests.
    pub fn recv_from(socket: &UdpSocket) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        let (number_of_bytes, from) = socket.recv_from(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok((packet, from))
    }

    /// Receives a data packet from the socket's connected remote, and returns the
    /// parsed [`Packet`]. The received packet can actually be of any type, however,
    /// this function also allows supplying the buffer size for an incoming request.
    pub fn recv_data(socket: &UdpSocket, size: usize) -> Result<Packet, Box<dyn Error>> {
        let mut buf = vec![0; size + 4];
        let number_of_bytes = socket.recv(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok(packet)
    }
}
