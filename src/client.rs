use crate::packet::{DEFAULT_BLOCKSIZE, DEFAULT_TIMEOUT, DEFAULT_WINDOWSIZE};
use crate::{ClientConfig, OptionType, Packet, Socket, TransferOption, Worker};
use std::cmp::PartialEq;
use std::error::Error;
use std::fs::File;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::time::Duration;

/// Client `struct` is used for client sided TFTP requests.
///
/// This `struct` is meant to be created by [`Client::new()`]. See its
/// documentation for more.
///
/// # Example
///
/// ```rust
/// // Create the TFTP server.
/// use tftpd::{ClientConfig, Client};
///
/// let args = ["test.file", "-u"].iter().map(|s| s.to_string());
/// let config = ClientConfig::new(args).unwrap();
/// let server = Client::new(&config).unwrap();
/// ```
pub struct Client {
    remote_address: SocketAddr,
    blocksize: usize,
    windowsize: u16,
    timeout: Duration,
    mode: Mode,
    filename: PathBuf,
    save_path: PathBuf,
}

/// Enum used to set the client either in Download Mode or Upload Mode
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    /// Upload Mode
    Upload,
    /// Download Mode
    Download,
}

impl Client {
    /// Creates the TFTP Client with the supplied [`ClientConfig`].
    pub fn new(config: &ClientConfig) -> Result<Client, Box<dyn Error>> {
        Ok(Client {
            remote_address: SocketAddr::from((config.remote_ip_address, config.port)),
            blocksize: config.blocksize,
            windowsize: config.windowsize,
            timeout: config.timeout,
            mode: config.mode,
            filename: config.filename.clone(),
            save_path: config.save_directory.clone(),
        })
    }

    /// Run the Client depending on the [`Mode`] the client is in
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        match self.mode {
            Mode::Upload => self.upload(),
            Mode::Download => self.download(),
        }
    }

    fn upload(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mode != Mode::Upload {
            return Err(Box::from("Client mode is set to Download"));
        }

        let socket = if self.remote_address.is_ipv4() {
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?
        } else {
            UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?
        };
        let file = self.filename.clone();

        let size = File::open(self.filename.clone())?.metadata()?.len() as usize;

        Socket::send_to(
            &socket,
            &Packet::Wrq {
                filename: file.into_os_string().into_string().unwrap(),
                mode: "octet".into(),
                options: vec![
                    TransferOption {
                        option: OptionType::BlockSize,
                        value: self.blocksize,
                    },
                    TransferOption {
                        option: OptionType::Windowsize,
                        value: self.windowsize as usize,
                    },
                    TransferOption {
                        option: OptionType::Timeout,
                        value: self.timeout.as_secs() as usize,
                    },
                    TransferOption {
                        option: OptionType::TransferSize,
                        value: size,
                    }
                ],
            },
            &self.remote_address,
        )?;

        let received = Socket::recv_from(&socket);

        if let Ok((packet, from)) = received {
            socket.connect(from)?;
            match packet {
                Packet::Oack(options) => {
                    self.verify_oack(&options)?;
                    let worker = self.configure_worker(socket)?;
                    let join_handle = worker.send(false)?;
                    let _ = join_handle.join();
                }
                Packet::Ack(_) => {
                    self.blocksize = DEFAULT_BLOCKSIZE;
                    self.windowsize = DEFAULT_WINDOWSIZE;
                    self.timeout = DEFAULT_TIMEOUT;
                    let worker = self.configure_worker(socket)?;
                    let join_handle = worker.send(false)?;
                    let _ = join_handle.join();
                }
                Packet::Error { code, msg } => {
                    return Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"
                    )));
                }
                _ => {
                    return Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"
                    )));
                }
            }
        } else {
            return Err(Box::from("Unexpected Error"));
        }

        Ok(())
    }

    fn download(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mode != Mode::Download {
            return Err(Box::from("Client mode is set to Upload"));
        }

        let socket = if self.remote_address.is_ipv4() {
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?
        } else {
            UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?
        };
        let file = self.filename.clone();

        Socket::send_to(
            &socket,
            &Packet::Rrq {
                filename: file.into_os_string().into_string().unwrap(),
                mode: "octet".into(),
                options: vec![
                    TransferOption {
                        option: OptionType::BlockSize,
                        value: self.blocksize,
                    },
                    TransferOption {
                        option: OptionType::Windowsize,
                        value: self.windowsize as usize,
                    },
                    TransferOption {
                        option: OptionType::Timeout,
                        value: self.timeout.as_secs() as usize,
                    },
                    TransferOption {
                        option: OptionType::TransferSize,
                        value: 0,
                    }
                ],
            },
            &self.remote_address,
        )?;

        let received = Socket::recv_from(&socket);

        if let Ok((packet, from)) = received {
            socket.connect(from)?;
            match packet {
                Packet::Oack(options) => {
                    self.verify_oack(&options)?;
                    Socket::send_to(&socket, &Packet::Ack(0), &from)?;
                    let worker = self.configure_worker(socket)?;
                    let join_handle = worker.receive()?;
                    let _ = join_handle.join();
                }
                Packet::Error { code, msg } => {
                    return Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"
                    )));
                }
                _ => {
                    return Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"
                    )));
                }
            }
        } else {
            return Err(Box::from("Unexpected Error"));
        }

        Ok(())
    }

    fn verify_oack(&mut self, options: &Vec<TransferOption>) -> Result<(), Box<dyn Error>> {
        for option in options {
            match option.option {
                OptionType::BlockSize {} => self.blocksize = option.value,
                OptionType::Windowsize => self.windowsize = option.value as u16,
                _ => {}
            }
        }

        Ok(())
    }

    fn configure_worker(&self, socket: UdpSocket) -> Result<Worker<dyn Socket>, Box<dyn Error>> {
        let mut socket: Box<dyn Socket> = Box::new(socket);

        socket.set_read_timeout(self.timeout)?;
        socket.set_write_timeout(self.timeout)?;

        let worker = if self.mode == Mode::Download {
            let mut file = self.save_path.clone();
            file = file.join(self.filename.clone());
            Worker::new(
                socket,
                file,
                self.blocksize,
                DEFAULT_TIMEOUT,
                self.windowsize,
                1,
            )
        } else {
            Worker::new(
                socket,
                PathBuf::from(self.filename.clone()),
                self.blocksize,
                DEFAULT_TIMEOUT,
                self.windowsize,
                1,
            )
        };

        Ok(worker)
    }
}
