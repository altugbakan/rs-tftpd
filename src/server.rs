use crate::{Config, Message, Worker};
use crate::{ErrorCode, Packet, TransferOption};
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};

/// Server `struct` is used for handling incoming TFTP requests.
///
/// This `struct` is meant to be created by [`Server::new()`]. See its
/// documentation for more.
///
/// # Example
///
/// ```rust
/// // Create the TFTP server.
/// use tftpd::{Config, Server};
///
/// let args = ["/", "-p", "1234"].iter().map(|s| s.to_string());
/// let config = Config::new(args).unwrap();
/// let server = Server::new(&config).unwrap();
/// ```
pub struct Server {
    socket: UdpSocket,
    directory: PathBuf,
}

impl Server {
    /// Creates the TFTP Server with the supplied [`Config`].
    pub fn new(config: &Config) -> Result<Server, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((config.ip_address, config.port)))?;

        let server = Server {
            socket,
            directory: config.directory.clone(),
        };

        Ok(server)
    }

    /// Starts listening for connections. Note that this function does not finish running until termination.
    pub fn listen(&self) {
        loop {
            if let Ok((packet, from)) = Message::recv_from(&self.socket) {
                match packet {
                    Packet::Rrq {
                        filename,
                        mut options,
                        ..
                    } => {
                        println!("Sending {filename} to {from}");
                        if let Err(err) = self.handle_rrq(filename.clone(), &mut options, &from) {
                            eprintln!("Error while sending file: {err}")
                        }
                    }
                    Packet::Wrq {
                        filename,
                        mut options,
                        ..
                    } => {
                        println!("Receiving {filename} from {from}");
                        if let Err(err) = self.handle_wrq(filename.clone(), &mut options, &from) {
                            eprintln!("Error while receiving file: {err}")
                        }
                    }
                    _ => {
                        Message::send_error_to(
                            &self.socket,
                            &from,
                            ErrorCode::IllegalOperation,
                            "invalid request",
                        )
                        .unwrap_or_else(|_| eprintln!("Received invalid request"));
                    }
                };
            }
        }
    }

    fn handle_rrq(
        &self,
        filename: String,
        options: &mut [TransferOption],
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let file_path = &self.directory.join(filename);
        match check_file_exists(file_path, &self.directory) {
            ErrorCode::FileNotFound => Message::send_error_to(
                &self.socket,
                to,
                ErrorCode::FileNotFound,
                "file does not exist",
            ),
            ErrorCode::AccessViolation => Message::send_error_to(
                &self.socket,
                to,
                ErrorCode::AccessViolation,
                "file access violation",
            ),
            ErrorCode::FileExists => {
                Worker::send(
                    self.socket.local_addr()?,
                    *to,
                    file_path.to_path_buf(),
                    options.to_vec(),
                );
                Ok(())
            }
            _ => Err("unexpected error code when checking file".into()),
        }
    }

    fn handle_wrq(
        &self,
        filename: String,
        options: &mut [TransferOption],
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let file_path = &self.directory.join(filename);
        match check_file_exists(file_path, &self.directory) {
            ErrorCode::FileExists => Message::send_error_to(
                &self.socket,
                to,
                ErrorCode::FileExists,
                "requested file already exists",
            ),
            ErrorCode::AccessViolation => Message::send_error_to(
                &self.socket,
                to,
                ErrorCode::AccessViolation,
                "file access violation",
            ),
            ErrorCode::FileNotFound => {
                Worker::receive(
                    self.socket.local_addr()?,
                    *to,
                    file_path.to_path_buf(),
                    options.to_vec(),
                );
                Ok(())
            }
            _ => Err("unexpected error code when checking file".into()),
        }
    }
}

fn check_file_exists(file: &Path, directory: &PathBuf) -> ErrorCode {
    if !validate_file_path(file, directory) {
        return ErrorCode::AccessViolation;
    }

    if !file.exists() {
        return ErrorCode::FileNotFound;
    }

    ErrorCode::FileExists
}

fn validate_file_path(file: &Path, directory: &PathBuf) -> bool {
    !file.to_str().unwrap().contains("..") && file.ancestors().any(|a| a == directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_file_path() {
        assert!(validate_file_path(
            &PathBuf::from("/dir/test/file"),
            &PathBuf::from("/dir/test")
        ));

        assert!(!validate_file_path(
            &PathBuf::from("/system/data.txt"),
            &PathBuf::from("/dir/test")
        ));

        assert!(!validate_file_path(
            &PathBuf::from("~/some_data.txt"),
            &PathBuf::from("/dir/test")
        ));

        assert!(!validate_file_path(
            &PathBuf::from("/dir/test/../file"),
            &PathBuf::from("/dir/test")
        ));
    }
}
