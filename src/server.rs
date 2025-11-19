use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[cfg(debug_assertions)]
use crate::options::OptionFmt;
use crate::options::{OptionsPrivate, OptionsProtocol, DEFAULT_BLOCK_SIZE};
use crate::{log::*, ServerSocket, Socket, TransferOption, Worker};
use crate::{Config, ErrorCode, Packet};

#[cfg(test)]
use crate::OptionType;

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
    receive_directory: PathBuf,
    send_directory: PathBuf,
    single_port: bool,
    read_only: bool,
    overwrite: bool,
    largest_block_size: u16,
    clients: HashMap<SocketAddr, Sender<Packet>>,
    opt_local: OptionsPrivate,
}

impl Server {
    /// Creates the TFTP Server with the supplied [`Config`].
    pub fn new(config: &Config) -> Result<Server, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((config.ip_address, config.port)))?;
        let server = Server {
            socket,
            receive_directory: config.receive_directory.clone(),
            send_directory: config.send_directory.clone(),
            single_port: config.single_port,
            read_only: config.read_only,
            overwrite: config.overwrite,
            largest_block_size: DEFAULT_BLOCK_SIZE,
            clients: HashMap::new(),
            opt_local: config.opt_local.clone(),
        };

        Ok(server)
    }

    /// Starts listening for connections. Note that this function does not finish running until termination.
    pub fn listen(&mut self) {
        loop {
            let received = if self.single_port {
                self.socket
                    .recv_from_with_size(self.largest_block_size as usize)
            } else {
                Socket::recv_from(&self.socket)
            };

            if let Ok((packet, from)) = received {
                match packet {
                    Packet::Rrq {
                        filename,
                        mut options,
                        ..
                    } => {
                        log_info!("Received Read request from {from}: {filename}");
                        if let Err(err) = self.handle_rrq(filename.clone(), &mut options, &from) {
                            log_err!("Error while sending file: {err}")
                        }
                    }
                    Packet::Wrq {
                        filename,
                        mut options,
                        ..
                    } => {
                        if self.read_only {
                            if Socket::send_to(
                                &self.socket,
                                &Packet::Error {
                                    code: ErrorCode::AccessViolation,
                                    msg: "server is read-only".to_string(),
                                },
                                &from,
                            )
                            .is_err()
                            {
                                log_err!("Could not send error packet");
                            };
                            log_warn!("Received write request while in read-only mode");
                            continue;
                        }
                        log_info!("Received Write request from {from}: {filename}");
                        if let Err(err) = self.handle_wrq(filename, &mut options, &from) {
                            log_err!("Error while receiving file: {err}")
                        }
                    }
                    _ => {
                        if self.route_packet(packet, &from).is_err() {
                            if Socket::send_to(
                                &self.socket,
                                &Packet::Error {
                                    code: ErrorCode::IllegalOperation,
                                    msg: "invalid request".to_string(),
                                },
                                &from,
                            )
                            .is_err()
                            {
                                log_err!("Could not send error packet");
                            };
                            log_warn!("Received invalid request");
                        }
                    }
                };
            }
        }
    }

    fn handle_rrq(
        &mut self,
        filename: String,
        options: &mut [TransferOption],
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let file_path = convert_file_path(&filename);
        let file_path = &self.send_directory.join(file_path);
        match check_file_exists(file_path, &self.send_directory) {
            ErrorCode::FileNotFound => {
                log_warn!("Cannot find requested file: {}", file_path.display());
                Socket::send_to(
                    &self.socket,
                    &Packet::Error {
                        code: ErrorCode::FileNotFound,
                        msg: format!("file {} does not exist", file_path.display()),
                    },
                    to,
                )
            }
            ErrorCode::AccessViolation => {
                log_warn!("Cannot access requested file: {}", file_path.display());
                Socket::send_to(
                    &self.socket,
                    &Packet::Error {
                        code: ErrorCode::AccessViolation,
                        msg: format!("file access violation: {}", file_path.display()),
                    },
                    to,
                )
            }
            ErrorCode::FileExists => {
                let worker_options = OptionsProtocol::parse(
                    options,
                    RequestType::Read(file_path.metadata()?.len()),
                )?;
                let mut socket: Box<dyn Socket>;

                if self.single_port {
                    let single_socket =
                        create_single_socket(&self.socket, to, worker_options.timeout)?;
                    self.clients.insert(*to, single_socket.sender());
                    self.largest_block_size =
                        max(self.largest_block_size, worker_options.block_size);

                    socket = Box::new(single_socket);
                } else {
                    socket = Box::new(create_multi_socket(&self.socket.local_addr()?, to)?);
                }

                socket.set_read_timeout(worker_options.timeout)?;
                socket.set_write_timeout(worker_options.timeout)?;

                log_dbg!("  Accepted options: {}", OptionFmt(options));

                accept_request(
                    &socket,
                    options,
                    RequestType::Read(file_path.metadata()?.len()),
                )?;

                let worker = Worker::new(
                    socket,
                    file_path.clone(),
                    self.opt_local.clone(),
                    worker_options.clone(),
                );
                worker.send(!options.is_empty())?;
                Ok(())
            }
            _ => Err("Unexpected error code when checking file".into()),
        }
    }

    fn handle_wrq(
        &mut self,
        filename: String,
        options: &mut [TransferOption],
        to: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let file_path = convert_file_path(&filename);
        let file_path = &self.receive_directory.join(file_path);
        let initialize_write = &mut || -> Result<(), Box<dyn Error>> {
            let worker_options = OptionsProtocol::parse(options, RequestType::Write)?;
            let mut socket: Box<dyn Socket>;

            if self.single_port {
                let single_socket = create_single_socket(&self.socket, to, worker_options.timeout)?;
                self.clients.insert(*to, single_socket.sender());
                self.largest_block_size = max(self.largest_block_size, worker_options.block_size);

                socket = Box::new(single_socket);
            } else {
                socket = Box::new(create_multi_socket(&self.socket.local_addr()?, to)?);
            }

            socket.set_read_timeout(worker_options.timeout)?;
            socket.set_write_timeout(worker_options.timeout)?;

            log_dbg!("  Accepted options: {}", OptionFmt(options));
            accept_request(&socket, options, RequestType::Write)?;

            let worker = Worker::new(
                socket,
                file_path.clone(),
                self.opt_local.clone(),
                worker_options.clone(),
            );
            worker.receive()?;
            Ok(())
        };

        match check_file_exists(file_path, &self.receive_directory) {
            ErrorCode::FileExists => {
                if self.overwrite {
                    initialize_write()
                } else {
                    log_err!("File {} already exists", file_path.display());
                    Socket::send_to(
                        &self.socket,
                        &Packet::Error {
                            code: ErrorCode::FileExists,
                            msg: "requested file already exists".to_string(),
                        },
                        to,
                    )
                }
            }
            ErrorCode::AccessViolation => {
                log_err!("Access violation detected for file {}", file_path.display());
                Socket::send_to(
                    &self.socket,
                    &Packet::Error {
                        code: ErrorCode::AccessViolation,
                        msg: format!("file access violation: {}", file_path.display()),
                    },
                    to,
                )
            }
            ErrorCode::FileNotFound => initialize_write(),
            _ => Err("Unexpected error code when checking file".into()),
        }
    }

    fn route_packet(&self, packet: Packet, to: &SocketAddr) -> Result<(), Box<dyn Error>> {
        if self.clients.contains_key(to) {
            self.clients[to].send(packet)?;
            Ok(())
        } else {
            Err("No client found for packet".into())
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RequestType {
    Read(u64),
    Write,
}

pub fn convert_file_path(filename: &str) -> PathBuf {
    let mut chars_filename = filename.chars();
    let nodrive_filename = if chars_filename.nth(1) == Some(':') {
        //nth() is consumming 2 firsts chars
        chars_filename.as_str()
    } else {
        filename
    };
    let formatted_filename = nodrive_filename.trim_start_matches(['/', '\\']).to_string();
    let normalized_filename = if MAIN_SEPARATOR == '\\' {
        formatted_filename.replace('/', "\\")
    } else {
        formatted_filename.replace('\\', "/")
    };

    PathBuf::from(normalized_filename)
}

fn create_single_socket(
    socket: &UdpSocket,
    remote: &SocketAddr,
    timeout: Duration,
) -> Result<ServerSocket, Box<dyn Error>> {
    let socket = ServerSocket::new(socket.try_clone()?, *remote, timeout);

    Ok(socket)
}

fn create_multi_socket(
    addr: &SocketAddr,
    remote: &SocketAddr,
) -> Result<UdpSocket, Box<dyn Error>> {
    let socket = UdpSocket::bind(SocketAddr::from((addr.ip(), 0)))?;
    socket.connect(remote)?;

    Ok(socket)
}

fn accept_request<T: Socket>(
    socket: &T,
    options: &[TransferOption],
    request_type: RequestType,
) -> Result<(), Box<dyn Error>> {
    if !options.is_empty() {
        socket.send(&Packet::Oack(options.to_vec()))?;
    } else if request_type == RequestType::Write {
        socket.send(&Packet::Ack(0))?;
    }

    Ok(())
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
    fn converts_file_path() {
        let path = convert_file_path("test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test.file");
        assert_eq!(path, correct_path);

        let path = convert_file_path("\\test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test.file");
        assert_eq!(path, correct_path);

        let path = convert_file_path("/test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test.file");
        assert_eq!(path, correct_path);

        let path = convert_file_path("C:\\test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test.file");
        assert_eq!(path, correct_path);

        let path = convert_file_path("test\\test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test");
        correct_path.push("test.file");
        assert_eq!(path, correct_path);

        let path = convert_file_path("test/test/test.file");
        let mut correct_path = PathBuf::new();
        correct_path.push("test");
        correct_path.push("test");
        correct_path.push("test.file");
        assert_eq!(path, correct_path);
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

    #[test]
    fn parses_write_options() {
        let mut options = vec![
            TransferOption {
                option: OptionType::BlockSize,
                value: 1024,
            },
            TransferOption {
                option: OptionType::TransferSize,
                value: 0,
            },
            TransferOption {
                option: OptionType::Timeout,
                value: 5,
            },
        ];

        let work_type = RequestType::Read(12341234);

        let worker_options = OptionsProtocol::parse(&mut options, work_type).unwrap();

        assert_eq!(options[0].value, worker_options.block_size as u64);
        assert_eq!(options[1].value, worker_options.transfer_size.unwrap());
        assert_eq!(options[2].value, worker_options.timeout.as_secs());
    }

    #[test]
    fn parses_read_options() {
        let mut options = vec![
            TransferOption {
                option: OptionType::BlockSize,
                value: 1024,
            },
            TransferOption {
                option: OptionType::TransferSize,
                value: 44554455,
            },
            TransferOption {
                option: OptionType::Timeout,
                value: 5,
            },
        ];

        let work_type = RequestType::Write;

        let worker_options = OptionsProtocol::parse(&mut options, work_type).unwrap();

        assert_eq!(options[0].value, worker_options.block_size as u64);
        assert_eq!(options[1].value, worker_options.transfer_size.unwrap());
        assert_eq!(options[2].value, worker_options.timeout.as_secs());
    }

    #[test]
    fn parses_default_options() {
        assert_eq!(
            OptionsProtocol::parse(&mut [], RequestType::Write).unwrap(),
            OptionsProtocol::default(),
        );
    }
}
