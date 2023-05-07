use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    time::Duration,
};

use crate::Packet;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Socket `trait` is used for easy message transmission of common TFTP
/// message types. This `trait` is implemented for [`UdpSocket`] and used
/// for abstraction of single socket communication.
pub trait Socket: Send + Sync + 'static {
    /// Sends a [`Packet`] to the socket's connected remote [`Socket`].
    fn send(&self, packet: &Packet) -> Result<(), Box<dyn Error>>;
    /// Sends a [`Packet`] to the specified remote [`Socket`].
    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> Result<(), Box<dyn Error>>;
    /// Receives a [`Packet`] from the socket's connected remote [`Socket`].
    fn recv(&self, buf: &mut [u8]) -> Result<Packet, Box<dyn Error>>;
    /// Receives a [`Packet`] from any remote [`Socket`] and returns the [`SocketAddr`]
    /// of the remote [`Socket`].
    fn recv_from(&self, buf: &mut [u8]) -> Result<(Packet, SocketAddr), Box<dyn Error>>;
    /// Returns the remote [`SocketAddr`] if it exists.
    fn remote_addr(&self) -> Result<SocketAddr, Box<dyn Error>>;
    /// Sets the read timeout for the [`Socket`].
    fn set_read_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>>;
    /// Sets the write timeout for the [`Socket`].
    fn set_write_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>>;
}

impl Socket for UdpSocket {
    fn send(&self, packet: &Packet) -> Result<(), Box<dyn Error>> {
        self.send(&packet.serialize()?)?;

        Ok(())
    }

    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> Result<(), Box<dyn Error>> {
        self.send_to(&packet.serialize()?, to)?;

        Ok(())
    }

    fn recv(&self, buf: &mut [u8]) -> Result<Packet, Box<dyn Error>> {
        let amt = self.recv(buf)?;
        let packet = Packet::deserialize(&buf[..amt])?;

        Ok(packet)
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        let (amt, addr) = self.recv_from(buf)?;
        let packet = Packet::deserialize(&buf[..amt])?;

        Ok((packet, addr))
    }

    fn remote_addr(&self) -> Result<SocketAddr, Box<dyn Error>> {
        Ok(self.peer_addr()?)
    }

    fn set_read_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>> {
        UdpSocket::set_read_timeout(self, Some(dur))?;

        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>> {
        UdpSocket::set_write_timeout(self, Some(dur))?;

        Ok(())
    }
}

/// ServerSocket `struct` is used as an abstraction layer for a server
/// [`Socket`]. This `struct` is used for abstraction of single socket
/// communication.
///
/// # Example
///
/// ```rust
/// use std::net::{SocketAddr, UdpSocket};
/// use std::str::FromStr;
/// use tftpd::{Socket, ServerSocket, Packet};
///
/// let socket = ServerSocket::new(
///     UdpSocket::bind("127.0.0.1:0").unwrap(),
///     SocketAddr::from_str("127.0.0.1:50000").unwrap(),
/// );
/// socket.send(&Packet::Ack(1)).unwrap();
/// ```
pub struct ServerSocket {
    socket: UdpSocket,
    remote: SocketAddr,
    sender: Mutex<Sender<Packet>>,
    receiver: Mutex<Receiver<Packet>>,
    timeout: Duration,
}

impl Socket for ServerSocket {
    fn send(&self, packet: &Packet) -> Result<(), Box<dyn Error>> {
        self.send_to(packet, &self.remote)
    }

    fn send_to(&self, packet: &Packet, to: &SocketAddr) -> Result<(), Box<dyn Error>> {
        self.socket.send_to(&packet.serialize()?, to)?;

        Ok(())
    }

    fn recv(&self, _buf: &mut [u8]) -> Result<Packet, Box<dyn Error>> {
        if let Ok(receiver) = self.receiver.lock() {
            if let Ok(packet) = receiver.recv_timeout(self.timeout) {
                Ok(packet)
            } else {
                Err("Failed to receive".into())
            }
        } else {
            Err("Failed to lock mutex".into())
        }
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        Ok((self.recv(buf)?, self.remote))
    }

    fn remote_addr(&self) -> Result<SocketAddr, Box<dyn Error>> {
        Ok(self.remote)
    }

    fn set_read_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>> {
        self.timeout = dur;

        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Duration) -> Result<(), Box<dyn Error>> {
        self.socket.set_write_timeout(Some(dur))?;

        Ok(())
    }
}

impl ServerSocket {
    /// Creates a new [`ServerSocket`] from a [`UdpSocket`] and a remote [`SocketAddr`].
    pub fn new(socket: UdpSocket, remote: SocketAddr) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            socket,
            remote,
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Returns a [`Sender`] for sending [`Packet`]s to the remote [`Socket`].
    pub fn sender(&self) -> Sender<Packet> {
        self.sender.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn test_recv() {
        let socket = ServerSocket::new(
            UdpSocket::bind("127.0.0.1:0").unwrap(),
            SocketAddr::from_str("127.0.0.1:50000").unwrap(),
        );

        socket.sender.lock().unwrap().send(Packet::Ack(1)).unwrap();

        let packet = socket.recv(&mut []).unwrap();

        assert_eq!(packet, Packet::Ack(1));

        socket
            .sender
            .lock()
            .unwrap()
            .send(Packet::Data {
                block_num: 15,
                data: vec![0x01, 0x02, 0x03],
            })
            .unwrap();

        let packet = socket.recv(&mut []).unwrap();

        assert_eq!(
            packet,
            Packet::Data {
                block_num: 15,
                data: vec![0x01, 0x02, 0x03]
            }
        );
    }
}
