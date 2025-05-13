use crate::{ErrorCode, Packet, Socket, Window};
use std::thread::JoinHandle;
use std::{
    error::Error,
    io::ErrorKind,
    fs::{self, File},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

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
/// use tftpd::Worker;
///
/// // Send a file, responding to a read request.
/// let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
/// socket.connect(SocketAddr::from_str("127.0.0.1:12345").unwrap()).unwrap();
/// let has_options = false;
///
/// let worker = Worker::new(
///     Box::new(socket),
///     PathBuf::from_str("Cargo.toml").unwrap(),
///     true,
///     512,
///     Duration::from_secs(1),
///     1,
///     Duration::from_millis(1),
///     1,
///     3,
/// );
///
/// worker.send(has_options).unwrap();
/// ```
pub struct Worker<T: Socket + ?Sized> {
    socket: Box<T>,
    file_path: PathBuf,
    clean_on_error: bool,
    blk_size: usize,
    timeout: Duration,
    window_size: u16,
    window_wait: Duration,
    repeat_amount: u8,
    max_retries: usize,
}

impl<T: Socket + ?Sized> Worker<T> {
    /// Creates a new [`Worker`] with the supplied options.
    pub fn new(
        socket: Box<T>,
        file_path: PathBuf,
        clean_on_error: bool,
        blk_size: usize,
        timeout: Duration,
        window_size: u16,
        window_wait: Duration,
        repeat_amount: u8,
        max_retries : usize,
    ) -> Worker<T> {
        Worker {
            socket,
            file_path,
            clean_on_error,
            blk_size,
            timeout,
            window_size,
            window_wait,
            repeat_amount,
            max_retries,
        }
    }

    /// Sends a file to the remote [`SocketAddr`] that has sent a read request using
    /// a random port, asynchronously.
    pub fn send(self, check_response: bool) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let file_path = self.file_path.clone();
        let remote_addr = self.socket.remote_addr().unwrap();

        let handle = thread::spawn(move || {
            let handle_send = || -> Result<(), Box<dyn Error>> {
                self.send_file(File::open(&file_path)?, check_response)?;

                Ok(())
            };

            match handle_send() {
                Ok(_) => {
                    println!(
                        "Sent {} to {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        &remote_addr
                    );
                }
                Err(err) => {
                    eprintln!(
                        "Error {err}, while sending {} to {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        &remote_addr
                    );
                }
            }
        });

        Ok(handle)
    }

    /// Receives a file from the remote [`SocketAddr`] (client or server) using
    /// the supplied socket, asynchronously.
    pub fn receive(self, transfer_size: usize) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let clean_on_error = self.clean_on_error;
        let file_path = self.file_path.clone();
        let remote_addr = self.socket.remote_addr().unwrap();

        let handle = thread::spawn(move || {
            let handle_receive = || -> Result<(), Box<dyn Error>> {
                self.receive_file(File::create(&file_path)?, transfer_size)?;

                Ok(())
            };

            match handle_receive() {
                Ok(_) => {
                    println!(
                        "Received {} from {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        remote_addr
                    );
                }
                Err(err) => {
                    eprintln!(
                        "Error {err}, while receiving {} from {}",
                        &file_path.file_name().unwrap().to_string_lossy(),
                        remote_addr
                    );
                    if clean_on_error && fs::remove_file(&file_path).is_err() {
                        eprintln!("Error while cleaning {}", &file_path.to_str().unwrap());
                    }
                }
            }
        });

        Ok(handle)
    }

    fn send_file(mut self, file: File, check_response: bool) -> Result<(), Box<dyn Error>> {
        let mut block_seq_win : u16 = 1;
        let mut win_idx : u16 = 0;
        let mut more = true;
        let mut window = Window::new(self.window_size, self.blk_size, file);

        let mut timeout_end = Instant::now();
        let mut retry_cnt = 0;

        self.socket.set_read_timeout(self.timeout)?;

        if check_response {
            self.check_response()?;
        }

        loop {
            if window.is_empty() {
                if !more {
                    return Ok(());
                }
                more = window.fill()?;
                self.socket.set_nonblocking(true)?;
            }

            if let Some(frame) = window.get_elements().get(win_idx as usize) {
                let block_seq_tx = block_seq_win.wrapping_add(win_idx);

                self.send_packet(&Packet::Data {
                    block_num: block_seq_tx,
                    data: frame.to_vec(),
                })?;
                win_idx += 1;

                if win_idx < window.len() {
                    if !self.window_wait.is_zero() {
                        thread::sleep(self.window_wait);
                    }
                } else {
                    self.socket.set_nonblocking(false)?;
                    timeout_end = Instant::now() + self.timeout;
                }
            }

            loop {
                match self.socket.recv() {
                    Ok(Packet::Ack(block_seq_rx)) => {

                        let next_seq = block_seq_rx.wrapping_add(1);
                        let diff = next_seq.wrapping_sub(block_seq_win);
                        if diff <= self.window_size {
                            block_seq_win = next_seq;
                            window.remove(diff)?;
                            win_idx = 0;
                            if diff != self.window_size && more {
                                more = window.fill()?;
                                self.socket.set_nonblocking(true)?;
                            }
                            break;
                        } else {
                            // Received ack w/ unexpected seq: probably old pkt out of order
                        }
                    }

                    Ok(Packet::Error{code, msg}) => return Err(format!("Received error code {code}: {msg}").into()),

                    Ok(_) => println!("Received unexpected packet"),

                    Err(e) => {
                        if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
                            match io_e.kind() {
                                /* On blocking sockets, Unix returns WouldBlock and Windows TimedOut */
                                ErrorKind::WouldBlock |
                                ErrorKind::TimedOut => if win_idx < window.len() {
                                    // Non blocking socket
                                    break;
                                } else {
                                    // Blocking socket, so timeout expired
                                    self.socket.set_nonblocking(true)?;
                                    win_idx = 0;
                                },
                                ErrorKind::ConnectionReset => println!("Cnx reset during reception {io_e:?}"),
                                _ => println!("IO error during reception {io_e:?}"),
                            }
                        } else {
                            println!("Unkown error during reception {e:?}");
                        }
                    }
                }

                if timeout_end < Instant::now() {
                    if retry_cnt == self.max_retries {
                        return Err(format!("Transfer timed out after {} tries", self.max_retries).into());
                    }
                    retry_cnt += 1;
                    timeout_end = Instant::now() + self.timeout;
                    break;
                }
            }
        }
    }

    fn receive_file(self, file: File, transfer_size: usize) -> Result<(), Box<dyn Error>> {
        let mut block_number: u16 = 0;
        let mut window = Window::new(self.window_size, self.blk_size, file);

        loop {
            let mut last = false;
            let mut retry_cnt = 0;

            loop {
                match self.socket.recv_with_size(self.blk_size) {
                    Ok(Packet::Data {
                        block_num: received_block_number,
                        data,
                    }) => {
                        if received_block_number == block_number.wrapping_add(1) {
                            block_number = received_block_number;
                            last = data.len() < self.blk_size;
                            window.add(data)?;

                            if window.is_full() || last {
                                break;
                            }
                        } else {
                            // Block number mismatch, send ack of last good block
                            break;
                        }
                    }
                    Ok(Packet::Error { code, msg }) => {
                        return Err(format!("Received error code {code}: {msg}").into());
                    }
                    _ => {
                        retry_cnt += 1;
                        if retry_cnt == self.max_retries {
                            return Err(
                                format!("Transfer timed out after {} tries", self.max_retries).into()
                            );
                        }
                    }
                }
            }

            window.empty()?;
            self.send_packet(&Packet::Ack(block_number))?;

            if last {
                if transfer_size != 0 && transfer_size != window.file_len()? {
                    return Err(format!("Size mismatch, negotiated: {}, transferred: {}",
                        transfer_size, window.file_len()?).into());
                }
                // we should wait and listen a bit more as per RFC 1350 section 6
                break;
            };
        }

        Ok(())
    }

    fn send_packet(&self, packet: &Packet) -> Result<(), Box<dyn Error>> {
        for i in 0..self.repeat_amount {
            if i > 0 {
                std::thread::sleep(DEFAULT_DUPLICATE_DELAY);
            }
            self.socket.send(packet)?;
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
