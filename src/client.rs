use crate::client_config::{DEFAULT_BLOCKSIZE, DEFAULT_TIMEOUT, DEFAULT_WINDOWSIZE};
use crate::{ClientConfig, OptionType, Packet, Socket, TransferOption, Worker};
use std::cmp::PartialEq;
use std::error::Error;
use std::fs;
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
    file_path: PathBuf,
    receive_directory: PathBuf,
    clean_on_error: bool,
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
            file_path: config.file_path.clone(),
            receive_directory: config.receive_directory.clone(),
            clean_on_error: config.clean_on_error,
        })
    }

    /// Run the Client depending on the [`Mode`] the client is in
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {

        let socket = if self.remote_address.is_ipv4() {
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?
        } else {
            UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?
        };

        match self.mode {
            Mode::Upload => self.upload(socket),
            Mode::Download => self.download(socket),
        }
    }

    fn prepare_options(&self, size : usize) -> Vec<TransferOption> {
        let mut options = vec![
            TransferOption {
                option: OptionType::BlockSize,
                value: self.blocksize,
            },
            TransferOption {
                option: OptionType::Windowsize,
                value: self.windowsize as usize,
            },
            TransferOption {
                option: OptionType::TransferSize,
                value: size,
            },
        ];

        options.push(if self.timeout.subsec_millis() == 0 {
            TransferOption {
                option: OptionType::Timeout,
                value: self.timeout.as_secs() as usize,
            }
        } else {
            TransferOption {
                option: OptionType::TimeoutMs,
                value: self.timeout.as_millis() as usize,
            }           
        });

        options
    }

    fn upload(&mut self, socket : UdpSocket) -> Result<(), Box<dyn Error>> {
        if self.mode != Mode::Upload {
            return Err(Box::from("Client mode is set to Download"));
        }

        let filename = self
            .file_path
            .file_name()
            .ok_or("Invalid filename")?
            .to_str()
            .ok_or("Filename is not valid UTF-8")?
            .to_owned();

        let size = fs::metadata(self.file_path.clone())?.len() as usize;

        Socket::send_to(
            &socket,
            &Packet::Wrq {
                filename,
                mode: "octet".into(),
                options : self.prepare_options(size),
            },
            &self.remote_address,
        )?;

        match Socket::recv_from(&socket) {
            Ok((packet, from)) => {
                socket.connect(from)?;
                match packet {
                    Packet::Oack(options) => {
                        self.verify_oack(&options)?;
                        let worker = self.configure_worker(socket)?;
                        let join_handle = worker.send(false)?;
                        let _ = join_handle.join();

                        Ok(())
                    }

                    Packet::Ack(_) => {
                        self.blocksize = DEFAULT_BLOCKSIZE;
                        self.windowsize = DEFAULT_WINDOWSIZE;
                        self.timeout = DEFAULT_TIMEOUT;
                        let worker = self.configure_worker(socket)?;
                        let join_handle = worker.send(false)?;
                        let _ = join_handle.join();

                        Ok(())
                    }

                    Packet::Error { code, msg } => Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"))),

                    _ => Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"))), 
                }
            }
            Err(err) => Err(Box::from(format!("Unexpected Error: {err}")))
        }
    }

    fn download(&mut self, socket : UdpSocket) -> Result<(), Box<dyn Error>> {
        if self.mode != Mode::Download {
            return Err(Box::from("Client mode is set to Upload"));
        }
        
        let filename = self
            .file_path
            .clone()
            .into_os_string()
            .into_string()
            .unwrap_or_else(|_| "Invalid filename".to_string());

        Socket::send_to(
            &socket,
            &Packet::Rrq {
                filename,
                mode: "octet".into(),
                options : self.prepare_options(0),
            },
            &self.remote_address,
        )?;

        match Socket::recv_from(&socket) {

            Ok((packet, from)) => {
                socket.connect(from)?;
                match packet {
                    Packet::Oack(options) => {
                        self.verify_oack(&options)?;
                        Socket::send_to(&socket, &Packet::Ack(0), &from)?;
                        let worker = self.configure_worker(socket)?;
                        let join_handle = worker.receive()?;
                        let _ = join_handle.join();

                        Ok(())
                    }
                    
                    Packet::Error { code, msg } => Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"))),

                    _ => Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"))),
                }               
            }
            Err(err) => Err(Box::from(format!("Unexpected Error: {err}")))
        }
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
            let mut file = self.receive_directory.clone();
            file = file.join(
                self.file_path
                    .clone()
                    .file_name()
                    .ok_or("Invalid filename")?,
            );
            Worker::new(
                socket,
                file,
                self.clean_on_error,
                self.blocksize,
                DEFAULT_TIMEOUT,
                self.windowsize,
                1,
            )
        } else {
            Worker::new(
                socket,
                self.file_path.clone(),
                self.clean_on_error,
                self.blocksize,
                DEFAULT_TIMEOUT,
                self.windowsize,
                1,
            )
        };

        Ok(worker)
    }
}
