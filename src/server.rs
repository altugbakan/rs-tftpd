use crate::packet::{ErrorCode, Packet, TransferOption};
use crate::{Config, Message, Worker};
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;

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
            if let Ok((packet, from)) = Message::recv_from(&self.socket) {
                match packet {
                    Packet::Rrq {
                        filename,
                        mut options,
                        ..
                    } => match self.handle_rrq(filename.clone(), &mut options, &from) {
                        Ok(_) => {
                            println!("Sending {filename} to {from}");
                        }
                        Err(err) => eprintln!("{err}"),
                    },
                    Packet::Wrq {
                        filename,
                        mut options,
                        ..
                    } => match self.handle_wrq(filename.clone(), &mut options, &from) {
                        Ok(_) => {
                            println!("Receiving {filename} from {from}");
                        }
                        Err(err) => eprintln!("{err}"),
                    },
                    _ => {
                        Message::send_error_to(
                            &self.socket,
                            &from,
                            ErrorCode::IllegalOperation,
                            "invalid request",
                        )
                        .unwrap_or_else(|err| eprintln!("{err}"));
                    }
                };
            }
        }
    }

    fn handle_rrq(
        &self,
        filename: String,
        options: &mut Vec<TransferOption>,
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        match check_file_exists(&get_full_path(&filename, &self.directory), &self.directory) {
            ErrorCode::FileNotFound => {
                return Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::FileNotFound,
                    "file does not exist",
                );
            }
            ErrorCode::AccessViolation => {
                return Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::AccessViolation,
                    "file access violation",
                );
            }
            ErrorCode::FileExists => Worker::send(
                self.socket.local_addr().unwrap(),
                *to,
                filename,
                options.to_vec(),
            ),
            _ => {}
        }

        Ok(())
    }

    fn handle_wrq(
        &self,
        filename: String,
        options: &mut Vec<TransferOption>,
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        match check_file_exists(&get_full_path(&filename, &self.directory), &self.directory) {
            ErrorCode::FileExists => {
                return Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::FileExists,
                    "requested file already exists",
                );
            }
            ErrorCode::AccessViolation => {
                return Message::send_error_to(
                    &self.socket,
                    to,
                    ErrorCode::AccessViolation,
                    "file access violation",
                );
            }
            ErrorCode::FileNotFound => Worker::receive(
                self.socket.local_addr().unwrap(),
                *to,
                filename,
                options.to_vec(),
            ),
            _ => {}
        };

        Ok(())
    }
}

fn check_file_exists(file: &PathBuf, directory: &PathBuf) -> ErrorCode {
    if !validate_file_path(file, directory) {
        return ErrorCode::AccessViolation;
    }

    if !file.exists() {
        return ErrorCode::FileNotFound;
    }

    ErrorCode::FileExists
}

fn validate_file_path(file: &PathBuf, directory: &PathBuf) -> bool {
    !file.to_str().unwrap().contains("..") && file.ancestors().any(|a| a == directory)
}

fn get_full_path(filename: &str, directory: &PathBuf) -> PathBuf {
    let mut file = directory.clone();
    file.push(PathBuf::from(filename));
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_full_path() {
        assert_eq!(
            get_full_path("test.txt", &PathBuf::from("/dir/test")),
            PathBuf::from("/dir/test/test.txt")
        );

        assert_eq!(
            get_full_path("some_dir/test.txt", &PathBuf::from("/dir/test")),
            PathBuf::from("/dir/test/some_dir/test.txt")
        );
    }

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
