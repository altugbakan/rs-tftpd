use crate::{ErrorCode, Packet, Socket, Window};
use std::thread::JoinHandle;
use std::{
    error::Error,
    fs::{self, File},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

const MAX_RETRIES: u32 = 6;
const TIMEOUT_BUFFER: Duration = Duration::from_secs(1);
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
///     512,
///     Duration::from_secs(1),
///     1,
///     1,
/// );
///
/// worker.send(has_options).unwrap();
/// ```
pub struct Worker<T: Socket + ?Sized> {
    socket: Box<T>,
    file_name: PathBuf,
    blk_size: usize,
    timeout: Duration,
    windowsize: u16,
    repeat_amount: u8,
}

impl<T: Socket + ?Sized> Worker<T> {
    /// Creates a new [`Worker`] with the supplied options.
    pub fn new(
        socket: Box<T>,
        file_name: PathBuf,
        blk_size: usize,
        timeout: Duration,
        windowsize: u16,
        repeat_amount: u8,
    ) -> Worker<T> {
        Worker {
            socket,
            file_name,
            blk_size,
            timeout,
            windowsize,
            repeat_amount,
        }
    }

    /// Sends a file to the remote [`SocketAddr`] that has sent a read request using
    /// a random port, asynchronously.
    pub fn send(self, check_response: bool) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let file_name = self.file_name.clone();
        let remote_addr = self.socket.remote_addr().unwrap();

        let handle = thread::spawn(move || {
            let handle_send = || -> Result<(), Box<dyn Error>> {
                self.send_file(File::open(&file_name)?, check_response)?;

                Ok(())
            };

            match handle_send() {
                Ok(_) => {
                    println!(
                        "Sent {} to {}",
                        &file_name.file_name().unwrap().to_string_lossy(),
                        &remote_addr
                    );
                }
                Err(err) => {
                    eprintln!("{err}");
                }
            }
        });

        Ok(handle)
    }

    /// Receives a file from the remote [`SocketAddr`] that has sent a write request using
    /// the supplied socket, asynchronously.
    pub fn receive(self) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let file_name = self.file_name.clone();
        let remote_addr = self.socket.remote_addr().unwrap();

        let handle = thread::spawn(move || {
            let handle_receive = || -> Result<(), Box<dyn Error>> {
                self.receive_file(File::create(&file_name)?)?;

                Ok(())
            };

            match handle_receive() {
                Ok(_) => {
                    println!(
                        "Received {} from {}",
                        &file_name.file_name().unwrap().to_string_lossy(),
                        remote_addr
                    );
                }
                Err(err) => {
                    eprintln!("{err}");
                    if fs::remove_file(&file_name).is_err() {
                        eprintln!("Error while cleaning {}", &file_name.to_str().unwrap());
                    }
                }
            }
        });

        Ok(handle)
    }

    fn send_file(self, file: File, check_response: bool) -> Result<(), Box<dyn Error>> {
        let mut block_number = 1;
        let mut window = Window::new(self.windowsize, self.blk_size, file);

        if check_response {
            self.check_response()?;
        }
        loop {
            let filled = window.fill()?;

            let mut retry_cnt = 0;
            let mut time = Instant::now() - (self.timeout + TIMEOUT_BUFFER);
            loop {
                if time.elapsed() >= self.timeout {
                    self.send_window(&window, block_number)?;
                    time = Instant::now();
                }

                match self.socket.recv() {
                    Ok(Packet::Ack(received_block_number)) => {
                        let diff = received_block_number.wrapping_sub(block_number);
                        if diff <= self.windowsize {
                            block_number = received_block_number.wrapping_add(1);
                            window.remove(diff + 1)?;
                            break;
                        }
                    }
                    Ok(Packet::Error { code, msg }) => {
                        return Err(format!("Received error code {code}: {msg}").into());
                    }
                    _ => {
                        retry_cnt += 1;
                        if retry_cnt == MAX_RETRIES {
                            return Err(
                                format!("Transfer timed out after {MAX_RETRIES} tries").into()
                            );
                        }
                    }
                }
            }

            if !filled && window.is_empty() {
                break;
            }
        }

        Ok(())
    }

    fn receive_file(self, file: File) -> Result<(), Box<dyn Error>> {
        let mut block_number: u16 = 0;
        let mut window = Window::new(self.windowsize, self.blk_size, file);

        loop {
            let mut size;
            let mut retry_cnt = 0;

            loop {
                match self.socket.recv_with_size(self.blk_size) {
                    Ok(Packet::Data {
                        block_num: received_block_number,
                        data,
                    }) => {
                        if received_block_number == block_number.wrapping_add(1) {
                            block_number = received_block_number;
                            size = data.len();
                            window.add(data)?;

                            if size < self.blk_size {
                                break;
                            }

                            if window.is_full() {
                                break;
                            }
                        }
                    }
                    Ok(Packet::Error { code, msg }) => {
                        return Err(format!("Received error code {code}: {msg}").into());
                    }
                    _ => {
                        retry_cnt += 1;
                        if retry_cnt == MAX_RETRIES {
                            return Err(
                                format!("Transfer timed out after {MAX_RETRIES} tries").into()
                            );
                        }
                    }
                }
            }

            window.empty()?;
            self.send_packet(&Packet::Ack(block_number))?;

            if size < self.blk_size {
                break;
            };
        }

        Ok(())
    }

    fn send_window(&self, window: &Window, mut block_num: u16) -> Result<(), Box<dyn Error>> {
        for frame in window.get_elements() {
            self.send_packet(&Packet::Data {
                block_num,
                data: frame.to_vec(),
            })?;
            block_num = block_num.wrapping_add(1);
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
        if let Packet::Ack(received_block_number) = self.socket.recv()? {
            if received_block_number != 0 {
                self.socket.send(&Packet::Error {
                    code: ErrorCode::IllegalOperation,
                    msg: "invalid oack response".to_string(),
                })?;
            }
        }

        Ok(())
    }
}
