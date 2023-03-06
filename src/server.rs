use crate::packet::{ErrorCode, Opcode, Packet};
use crate::Config;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};

const MAX_REQUEST_PACKET_SIZE: usize = 512;

pub struct Server {
    socket: UdpSocket,
}

impl Server {
    pub fn new(config: &Config) -> Result<Server, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((config.ip_address, config.port)))?;

        let server = Server { socket };

        Ok(server)
    }

    pub fn listen(&self) {
        loop {
            let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
            if let Ok((number_of_bytes, from)) = self.socket.recv_from(&mut buf) {
                if let Ok(packet) = Packet::deserialize(&buf[..number_of_bytes]) {
                    match packet {
                        Packet::Rrq {
                            filename,
                            mode,
                            options,
                        } => todo!(),
                        Packet::Wrq {
                            filename,
                            mode,
                            options,
                        } => todo!(),
                        _ => self.send_error_msg(from),
                    }
                };
            }
        }
    }

    fn send_error_msg(&self, to: SocketAddr) {
        let buf = [
            0x00,
            Opcode::Error as u8,
            0x00,
            ErrorCode::IllegalOperation as u8,
            0x00,
        ];
        self.socket.send_to(&buf, to);
    }
}
