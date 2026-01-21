use std::cmp::PartialEq;
use std::error::Error;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(debug_assertions)]
use crate::options::OptionFmt;
use crate::options::{OptionsPrivate, OptionsProtocol};
use crate::{log::*, ClientConfig, Packet, Socket, Worker};

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
    timeout_req: Duration,
    mode: Mode,
    file_local: PathBuf,
    file_remote: String,
    receive_directory: PathBuf,
    opt_local: OptionsPrivate,
    opt_common: OptionsProtocol,
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
            timeout_req: config.timeout_req,
            mode: config.mode,
            file_local: config.file_path.clone(),
            file_remote: config.file_remote.clone(),
            receive_directory: config.receive_directory.clone(),
            opt_local: config.opt_local.clone(),
            opt_common: config.opt_common.clone(),
        })
    }

    /// Run the Client depending on the [`Mode`] the client is in
    pub fn run(&mut self) -> Result<bool, Box<dyn Error>> {
        let socket = if self.remote_address.is_ipv4() {
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?
        } else {
            UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?
        };

        socket.set_read_timeout(Some(self.timeout_req))?;

        match self.mode {
            Mode::Upload => self.upload(socket),
            Mode::Download => self.download(socket),
        }
    }

    fn upload(&mut self, socket: UdpSocket) -> Result<bool, Box<dyn Error>> {
        if self.mode != Mode::Upload {
            return Err(Box::from("Client mode is set to Download"));
        }

        if self.file_remote.is_empty() {
            // 1 path provided: use filename as remote
            self.file_remote = self
                .file_local
                .file_name()
                .ok_or("Invalid filename")?
                .display()
                .to_string();
        }

        self.opt_common.transfer_size = Some(fs::metadata(self.file_local.clone())?.len());

        log_dbg!("  Sending Write request for {}", self.file_remote);
        Socket::send_to(
            &socket,
            &Packet::Wrq {
                filename : self.file_remote.clone(),
                mode: "octet".into(),
                options: self.opt_common.prepare(),
            },
            &self.remote_address,
        )?;

        match Socket::recv_from(&socket) {
            Ok((packet, from)) => {
                socket.connect(from)?;
                match packet {
                    Packet::Oack(options) => {
                        // Reset options before applying those from server
                        self.opt_common = Default::default();
                        self.opt_common.apply(&options)?;
                        log_dbg!("  Accepted options: {}", OptionFmt(&options));
                    }

                    Packet::Ack(_) => {
                        self.opt_common = Default::default();
                        log_dbg!("  Options not accepted, using default");
                    }

                    Packet::Error { code, msg } => return Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"
                    ))),

                    _ => return Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"
                    ))),
                }

                let worker = self.configure_worker(socket)?;
                let join_handle = worker.send(false)?;
                Ok(join_handle.join().unwrap())
            }
            Err(err) => Err(Box::from(format!("Unexpected Error: {err}"))),
        }
    }

    fn download(&mut self, socket: UdpSocket) -> Result<bool, Box<dyn Error>> {
        if self.mode != Mode::Download {
            return Err(Box::from("Client mode is set to Upload"));
        }

        if self.file_remote.is_empty() {
            // 1 path provided: use it as remote and use rxdir + filename as local
            self.file_remote = self
                .file_local
                .display()
                .to_string();
            self.file_local = self.receive_directory.join(self
                .file_local
                .file_name()
                .ok_or("Invalid filename")?)
        } else {
            // 2 paths provided: prefix the local one with rxdir and use remote as is
            self.file_local = self.receive_directory.join(self.file_local.clone());
        }

        log_dbg!("  Sending Read request for {}", self.file_remote);
        Socket::send_to(
            &socket,
            &Packet::Rrq {
                filename : self.file_remote.clone(),
                mode: "octet".into(),
                options: self.opt_common.prepare(),
            },
            &self.remote_address,
        )?;

        match Socket::recv_from(&socket) {
            Ok((packet, from)) => {
                socket.connect(from)?;
                match packet {
                    Packet::Oack(options) => {
                        // Reset options before applying those from server
                        self.opt_common = Default::default();
                        self.opt_common.apply(&options)?;
                        log_dbg!("  Accepted options: {}", OptionFmt(&options));
                        Socket::send_to(&socket, &Packet::Ack(0), &from)?;
                        let worker = self.configure_worker(socket)?;
                        let join_handle = worker.receive()?;
                        Ok(join_handle.join().unwrap())
                    }

                    // We could implement this by forwarding Option<packet::Data> to worker.receive()
                    Packet::Data { .. } => Err(
                        "Client received data instead of o-ack. This implementation \
                        does not support servers without options (RFC 2347)"
                            .into(),
                    ),

                    Packet::Error { code, msg } => Err(Box::from(format!(
                        "Client received error from server: {code}: {msg}"
                    ))),

                    _ => Err(Box::from(format!(
                        "Client received unexpected packet from server: {packet:#?}"
                    ))),
                }
            }
            Err(err) => Err(Box::from(format!("Unexpected Error: {err}"))),
        }
    }

    fn configure_worker(&self, socket: UdpSocket) -> Result<Worker<dyn Socket>, Box<dyn Error>> {
        let mut socket: Box<dyn Socket> = Box::new(socket);

        socket.set_read_timeout(self.opt_common.timeout)?;
        socket.set_write_timeout(self.opt_common.timeout)?;

        Ok(Worker::new(
            socket,
            self.file_local.clone(),
            self.opt_local.clone(),
            self.opt_common.clone(),
        ))
    }
}
