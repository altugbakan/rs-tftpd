use crate::packet::{ErrorCode, Packet, TransferOption};
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
                    self.handle_packet(&packet, &from)
                }
            }
        }
    }

    fn handle_packet(&self, packet: &Packet, from: &SocketAddr) {
        match &packet {
            Packet::Rrq {
                filename, options, ..
            } => self.validate_rrq(filename, options, from),
            Packet::Wrq {
                filename, options, ..
            } => self.validate_wrq(filename, options, from),
            _ => {
                Message::send_error_to(
                    &self.socket,
                    from,
                    ErrorCode::IllegalOperation,
                    "invalid request",
                );
            }
        }
    }

    fn validate_rrq(&self, filename: &String, options: &Vec<TransferOption>, to: &SocketAddr) {
        match self.check_file_exists(&Path::new(&filename)) {
            ErrorCode::FileNotFound => {
                Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::FileNotFound,
                    "requested file does not exist",
                );
            }
            ErrorCode::AccessViolation => {
                Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::AccessViolation,
                    "requested file is not in the directory",
                );
            }
            ErrorCode::FileExists => self
                .handle_rrq(filename, options, to)
                .unwrap_or_else(|err| eprintln!("could not handle read request: {err}")),
            _ => {}
        }
    }

    fn validate_wrq(&self, filename: &String, options: &Vec<TransferOption>, to: &SocketAddr) {
        match self.check_file_exists(&Path::new(&filename)) {
            ErrorCode::FileExists => {
                Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::FileExists,
                    "requested file already exists",
                );
            }
            ErrorCode::AccessViolation => {
                Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::AccessViolation,
                    "requested file is not in the directory",
                );
            }
            ErrorCode::FileNotFound => self
                .handle_wrq(filename, options, to)
                .unwrap_or_else(|err| eprintln!("could not handle write request: {err}")),
            _ => {}
        };
    }

    fn handle_rrq(
        &self,
        filename: &String,
        options: &Vec<TransferOption>,
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let mut worker = Worker::new(&self.socket.local_addr().unwrap(), to)?;
        worker.send_file(Path::new(&filename), options)?;

        Ok(())
    }

    fn handle_wrq(
        &self,
        filename: &String,
        options: &Vec<TransferOption>,
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let mut worker = Worker::new(&self.socket.local_addr().unwrap(), to)?;
        worker.receive_file(Path::new(&filename), options)?;

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
