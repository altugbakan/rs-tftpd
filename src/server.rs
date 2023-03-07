use crate::packet::{ErrorCode, Packet};
use crate::{Config, Message, Worker};
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};

const MAX_REQUEST_PACKET_SIZE: usize = 512;

pub struct Server {
    socket: UdpSocket,
    directory: PathBuf,
}

impl Server {
    pub fn new(config: &Config) -> Result<Server, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((config.ip_address, config.port)))?;

        let server = Server {
            socket,
            directory: config.directory.clone(),
        };

        Ok(server)
    }

    pub fn listen(&self) {
        loop {
            let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
            if let Ok((number_of_bytes, from)) = self.socket.recv_from(&mut buf) {
                if let Ok(packet) = Packet::deserialize(&buf[..number_of_bytes]) {
                    match &packet {
                        Packet::Rrq { filename, .. } => {
                            match self.check_file_exists(&Path::new(&filename)) {
                                ErrorCode::FileNotFound => {
                                    eprintln!("requested file does not exist");
                                    Message::send_error_to(
                                        &self.socket,
                                        &from,
                                        ErrorCode::FileNotFound,
                                        "requested file does not exist",
                                    );
                                }
                                ErrorCode::AccessViolation => {
                                    eprintln!("requested file is not in the directory");
                                    Message::send_error_to(
                                        &self.socket,
                                        &from,
                                        ErrorCode::AccessViolation,
                                        "requested file is not in the directory",
                                    );
                                }
                                ErrorCode::FileExists => self
                                    .handle_rrq(&packet, from)
                                    .unwrap_or_else(|_| eprintln!("could not handle read request")),
                                _ => {}
                            }
                        }
                        Packet::Wrq { filename, .. } => {
                            match self.check_file_exists(&Path::new(&filename)) {
                                ErrorCode::FileExists => {
                                    eprintln!("requested file already exists");
                                    Message::send_error_to(
                                        &self.socket,
                                        &from,
                                        ErrorCode::FileExists,
                                        "requested file already exists",
                                    );
                                }
                                ErrorCode::AccessViolation => {
                                    eprintln!("requested file is not in the directory");
                                    Message::send_error_to(
                                        &self.socket,
                                        &from,
                                        ErrorCode::AccessViolation,
                                        "requested file is not in the directory",
                                    );
                                }
                                ErrorCode::FileNotFound => {
                                    self.handle_wrq(&packet, from).unwrap_or_else(|_| {
                                        eprintln!("could not handle write request")
                                    })
                                }
                                _ => {}
                            }
                        }
                        _ => {
                            eprintln!("invalid request packet received");
                            Message::send_error_to(
                                &self.socket,
                                &from,
                                ErrorCode::IllegalOperation,
                                "invalid request",
                            );
                        }
                    }
                };
            }
        }
    }

    fn handle_rrq(&self, packet: &Packet, to: SocketAddr) -> Result<(), Box<dyn Error>> {
        if let Packet::Rrq {
            filename,
            mode: _,
            options,
        } = packet
        {
            let worker = Worker::new(self.socket.local_addr().unwrap(), to)?;
            worker.send_file(Path::new(&filename), options)?;
        } else {
            return Err("invalid read request packet".into());
        }

        Ok(())
    }

    fn handle_wrq(&self, packet: &Packet, to: SocketAddr) -> Result<(), Box<dyn Error>> {
        if let Packet::Wrq {
            filename,
            mode: _,
            options,
        } = packet
        {
            let worker = Worker::new(self.socket.local_addr().unwrap(), to)?;
            worker.receive_file(Path::new(&filename), options)?;
        } else {
            return Err("invalid write request packet".into());
        }

        Ok(())
    }

    fn check_file_exists(&self, file: &Path) -> ErrorCode {
        if !file.ancestors().any(|a| a == &self.directory) {
            return ErrorCode::AccessViolation;
        }

        if !file.exists() {
            return ErrorCode::FileNotFound;
        }

        ErrorCode::FileExists
    }
}
