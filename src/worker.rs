use std::{
    error::Error,
    fs::{self, File},
    io::ErrorKind,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use crate::log::*;
use crate::options::{OptionsPrivate, OptionsProtocol, Rollover};
use crate::{ErrorCode, Packet, Socket, WindowRead, WindowWrite};

#[cfg(feature = "debug_drop")]
use crate::drop::drop_check;

const DEFAULT_DUPLICATE_DELAY: Duration = Duration::from_millis(1);

/// Worker `struct` is used for multithreaded file sending and receiving.
/// It creates a new socket using the Server's IP and a random port
/// requested from the OS to communicate with the requesting client.
///
/// See [`Worker::send()`] and [`Worker::receive()`] for more details.
///
/// # Example
///
/// ```rust
/// use std::{net::{UdpSocket, SocketAddr}, path::PathBuf, str::FromStr, time::Duration};
/// use tftpd::{Worker};
///
/// // Send a file, responding to a read request.
/// let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
/// socket.connect(SocketAddr::from_str("127.0.0.1:12345").unwrap()).unwrap();
/// let has_options = false;
///
/// let worker = Worker::new(
///     Box::new(socket),
///     PathBuf::from_str("Cargo.toml").unwrap(),
///     Default::default(),
///     Default::default(),
/// );
///
/// worker.send(has_options).unwrap();
/// ```
pub struct Worker<T: Socket + ?Sized> {
    socket: Box<T>,
    file_path: PathBuf,
    opt_local: OptionsPrivate,
    opt_common: OptionsProtocol,
}

impl<T: Socket + ?Sized> Worker<T> {
    /// Creates a new [`Worker`] with the supplied options.
    pub fn new(
        socket: Box<T>,
        file_path: PathBuf,
        opt_local: OptionsPrivate,
        opt_common: OptionsProtocol,
    ) -> Worker<T> {
        Worker {
            socket,
            file_path,
            opt_local,
            opt_common,
        }
    }

    /// Sends a file to the remote [`SocketAddr`] that has sent a read request using
    /// a random port, asynchronously.
    pub fn send(self, check_response: bool) -> Result<thread::JoinHandle<bool>, Box<dyn Error>> {
        let file_path = self.file_path.clone();
        let remote_addr = self.socket.remote_addr().unwrap();

        let handle = thread::spawn(move || {
            let handle_send = || -> Result<(), Box<dyn Error>> {
                self.send_file(File::open(&file_path)?, check_response)
            };

            match handle_send() {
                Ok(_) => {
                    log_info!(
                        "Sent {} to {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        &remote_addr
                    );
                    true
                }
                Err(err) => {
                    log_err!(
                        "Error \"{err}\", while sending {} to {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        &remote_addr
                    );
                    false
                }
            }
        });

        Ok(handle)
    }

    /// Receives a file from the remote [`SocketAddr`] (client or server) using
    /// the supplied socket, asynchronously.
    pub fn receive(self) -> Result<thread::JoinHandle<bool>, Box<dyn Error>> {
        let clean_on_error = self.opt_local.clean_on_error;
        let file_path = self.file_path.clone();
        let remote_addr = self.socket.remote_addr().unwrap();
        let opt_tsize = self.opt_common.transfer_size;

        let handle = thread::spawn(move || {
            let handle_receive =
                || -> Result<u64, Box<dyn Error>> { self.receive_file(File::create(&file_path)?) };

            match handle_receive() {
                Ok(size) => {
                    if let Some(tsize) = opt_tsize {
                        if tsize != size {
                            log_err!("Size mismatch, negotiated: {tsize}, transferred: {size}");
                            return false;
                        }
                    }

                    log_info!(
                        "Received {} ({} bytes) from {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        size,
                        remote_addr
                    );
                    true
                }
                Err(err) => {
                    log_err!(
                        "Error \"{err}\", while receiving {} from {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        remote_addr
                    );
                    if clean_on_error && fs::remove_file(&file_path).is_err() {
                        log_err!("Error while cleaning {}", &file_path.to_str().unwrap());
                    }
                    false
                }
            }
        });

        Ok(handle)
    }

    fn send_file(mut self, file: File, check_response: bool) -> Result<(), Box<dyn Error>> {
        let mut block_seq_win: u16 = 0;
        let mut win_idx: u16 = 0;
        let mut window = WindowRead::new(
            self.opt_common.window_size,
            self.opt_common.block_size,
            file,
        );
        let mut more = window.fill()?;

        let mut timeout_end = Instant::now() + self.opt_common.timeout;
        let mut retry_cnt = 0;

        if cfg!(windows) {
            // On Windows, recv can return up to 15ms before timeout
            self.socket
                .set_read_timeout(self.opt_common.timeout + Duration::from_millis(15))?;
        } else if cfg!(unix) {
            self.socket.set_read_timeout(self.opt_common.timeout)?;
        }

        if check_response {
            self.check_response()?;
        }

        self.socket.set_nonblocking(true)?;

        loop {
            if let Some(frame) = window.get_elements().get(win_idx as usize) {
                let mut block_seq_tx = block_seq_win.wrapping_add(win_idx + 1);
                if block_seq_tx < block_seq_win {
                    match self.opt_local.rollover {
                        Rollover::None => return Err(self.send_rollover_error()),
                        Rollover::Enforce0 | Rollover::DontCare => (),
                        Rollover::Enforce1 => block_seq_tx += 1,
                    }
                }

                self.send_packet(&Packet::Data {
                    block_num: block_seq_tx,
                    data: frame.to_vec(),
                })?;
                win_idx += 1;

                if win_idx < window.len() {
                    if !self.opt_common.window_wait.is_zero() {
                        thread::sleep(self.opt_common.window_wait);
                    }
                } else {
                    window.prefill()?;
                    self.socket.set_nonblocking(false)?;
                    timeout_end = Instant::now() + self.opt_common.timeout;
                }
            }

            let mut last_ack: Option<u16> = None;
            loop {
                match self.socket.recv() {
                    Ok(Packet::Ack(block_seq_rx)) => {
                        if last_ack.is_none() {
                            self.socket.set_nonblocking(true)?;
                        }
                        last_ack = Some(block_seq_rx);
                        continue;
                    }

                    Ok(Packet::Error { code, msg }) => {
                        return Err(format!("Received error code {code}: {msg}").into())
                    }

                    Ok(_) => log_info!("  Received unexpected packet"),

                    Err(e) => {
                        if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
                            match io_e.kind() {
                                /* On non-blocking sockets, Windows returns WouldBlock and Unix TimedOut */
                                ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                                    if let Some(ack) = last_ack {
                                        let mut diff = ack.wrapping_sub(block_seq_win);
                                        if ack < block_seq_win
                                            && self.opt_local.rollover == Rollover::Enforce1
                                        {
                                            diff -= 1;
                                        }

                                        if diff == 0 {
                                            break;
                                        } else if diff <= self.opt_common.window_size {
                                            block_seq_win = ack;
                                            window.remove(diff)?;
                                            if !more && window.is_empty() {
                                                return Ok(());
                                            }
                                            more = more && window.fill()?;
                                            win_idx = 0;
                                            break;
                                        } else {
                                            log_dbg!("      Received Ack with unexpected seq {ack} (prev {block_seq_win})");
                                        }
                                    }
                                    if win_idx < window.len() && Instant::now() < timeout_end {
                                        break;
                                    }
                                }
                                ErrorKind::ConnectionReset => {
                                    log_info!("  Cnx reset during reception {io_e:?}")
                                }
                                _ => log_warn!("  IO error during reception {io_e:?}"),
                            }
                        } else {
                            log_warn!("  Unkown error during reception {e:?}");
                        }
                    }
                }

                if timeout_end < Instant::now() {
                    log_info!("  Ack timeout {}/{}", retry_cnt, self.opt_local.max_retries);
                    if retry_cnt == self.opt_local.max_retries {
                        return Err(format!(
                            "Transfer timed out after {} tries",
                            self.opt_local.max_retries
                        )
                        .into());
                    }
                    retry_cnt += 1;
                    timeout_end = Instant::now() + self.opt_common.timeout;
                    win_idx = 0;
                    self.socket.set_nonblocking(true)?;
                    break;
                }
            }
        }
    }

    fn send_rollover_error(&self) -> Box<dyn Error> {
        self.send_packet(&Packet::Error {
            code: ErrorCode::IllegalOperation,
            msg: "Block counter rollover error".to_string(),
        })
        .unwrap_or_else(|err| {
            log_err!("Error: error '{err:?}' while sending error code");
        });
        "Block counter rollover error".into()
    }

    fn receive_file(mut self, file: File) -> Result<u64, Box<dyn Error>> {
        let mut block_number: u16 = 0;
        let mut window = WindowWrite::new(
            self.opt_common.window_size,
            file,
        );
        let mut retry_cnt = 0;

        let mut last = false;
        let mut listen_all = false;
        let mut send_ack = false;

        while !last {
            while !send_ack {
                match self
                    .socket
                    .recv_with_size(self.opt_common.block_size as usize)
                {
                    Ok(Packet::Data {
                        block_num: received_block_number,
                        data,
                    }) => {
                        let mut new_block_number = block_number.wrapping_add(1);
                        if new_block_number == 0 {
                            match self.opt_local.rollover {
                                Rollover::None => return Err(self.send_rollover_error()),
                                Rollover::Enforce0 => {
                                    if received_block_number == 1 {
                                        log_warn!("  Warning: data packet 0 missed. Possible rollover policy mismatch.");
                                    }
                                }
                                Rollover::Enforce1 => {
                                    new_block_number = 1;
                                    if received_block_number == 0 {
                                        return Err(self.send_rollover_error());
                                    }
                                }
                                Rollover::DontCare => {
                                    if received_block_number == 1 {
                                        // Possible data loss if previous packet was 0 and lost
                                        log_dbg!("  Data packet 0 missed. Possible data loss.");
                                        new_block_number = 1;
                                    }
                                }
                            }
                        }

                        if received_block_number == new_block_number {
                            block_number = received_block_number;
                            last = data.len() < self.opt_common.block_size as usize;
                            window.add(data)?;
                            send_ack = window.is_full() || last;
                        } else {
                            log_dbg!("  Data packet mismatch. Received {received_block_number} instead of {new_block_number}.");
                            send_ack = true;
                        }

                        self.socket.set_nonblocking(true)?;
                        listen_all = true;
                    }
                    Ok(Packet::Error { code, msg }) => {
                        return Err(format!("Received error '{code}': {msg}").into());
                    }
                    Ok(_) => log_info!("  Received unexpected packet"),

                    Err(e) => {
                        if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
                            match io_e.kind() {
                                ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                                    if listen_all {
                                        self.socket.set_nonblocking(false)?;
                                        listen_all = false;
                                    } else {
                                        log_dbg!(
                                            "  Ack timeout {}/{}",
                                            retry_cnt,
                                            self.opt_local.max_retries
                                        );
                                        if retry_cnt == self.opt_local.max_retries {
                                            return Err(format!(
                                                "Transfer timed out after {} tries",
                                                self.opt_local.max_retries
                                            )
                                            .into());
                                        }
                                        retry_cnt += 1;
                                        send_ack = true;
                                    }
                                }
                                ErrorKind::ConnectionReset => {
                                    log_info!("  Cnx reset during reception {io_e:?}");
                                    self.socket.set_nonblocking(false)?;
                                }
                                _ => log_warn!("  IO error during reception {io_e:?}"),
                            }
                        } else {
                            log_warn!("  Unkown error during reception {e:?}");
                        }
                    }
                }
            }

            self.send_packet(&Packet::Ack(block_number))?;
            send_ack = false;

            window.empty()?;
        }

        // we should wait and listen a bit more as per RFC 1350 section 6

        window.file_len()
    }

    fn send_packet(&self, packet: &Packet) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "debug_drop")]
        if drop_check(packet) {
            return Ok(());
        };

        for i in 0..self.opt_local.repeat_count {
            if i > 0 {
                thread::sleep(DEFAULT_DUPLICATE_DELAY);
            }
            loop {
                match self.socket.send(packet) {
                    Ok(_) => break,
                    Err(e) => {
                        if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
                            if let ErrorKind::WouldBlock | ErrorKind::TimedOut = io_e.kind() {
                                thread::sleep(DEFAULT_DUPLICATE_DELAY);
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_response(&self) -> Result<(), Box<dyn Error>> {
        let pkt = self.socket.recv()?;
        if let Packet::Ack(received_block_number) = pkt {
            if received_block_number == 0 {
                return Ok(());
            }
        }

        self.socket.send(&Packet::Error {
            code: ErrorCode::IllegalOperation,
            msg: "invalid oack response".to_string(),
        })?;

        Err(format!("Unexpected packet received instead of Ack(0): {pkt:#?}").into())
    }
}
