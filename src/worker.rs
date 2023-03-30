use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};

use crate::{ErrorCode, Message, OptionType, Packet, TransferOption, Window};

/// Worker `struct` is used for multithreaded file sending and receiving.
/// It creates a new socket using the Server's IP and a random port
/// requested from the OS to communicate with the requesting client.
///
/// See [`Worker::send()`] and [`Worker::receive()`] for more details.
///
/// # Example
///
/// ```rust
/// use std::{net::SocketAddr, path::PathBuf, str::FromStr};
/// use tftpd::Worker;
///
/// // Send a file, responding to a read request.
/// Worker::send(
///     SocketAddr::from_str("127.0.0.1:1234").unwrap(),
///     SocketAddr::from_str("127.0.0.1:4321").unwrap(),
///     PathBuf::from_str("/home/rust/test.txt").unwrap(),
///     vec![]
/// );
/// ```
pub struct Worker;

#[derive(Debug, PartialEq, Eq)]
struct WorkerOptions {
    blk_size: usize,
    t_size: usize,
    timeout: u64,
    windowsize: u16,
}

#[derive(PartialEq, Eq)]
enum WorkType {
    Receive,
    Send(u64),
}

const MAX_RETRIES: u32 = 6;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_BLOCK_SIZE: usize = 512;

impl Worker {
    /// Sends a file to the remote [`SocketAddr`] that has sent a read request using
    /// a random port, asynchronously.
    pub fn send(
        addr: SocketAddr,
        remote: SocketAddr,
        file_path: PathBuf,
        mut options: Vec<TransferOption>,
    ) {
        thread::spawn(move || {
            let mut handle_send = || -> Result<(), Box<dyn Error>> {
                let socket = setup_socket(&addr, &remote)?;
                let work_type = WorkType::Send(file_path.metadata()?.len());
                let worker_options = parse_options(&mut options, &work_type)?;

                accept_request(&socket, &options, &work_type)?;
                send_file(&socket, File::open(&file_path)?, &worker_options)?;

                Ok(())
            };

            match handle_send() {
                Ok(_) => {
                    println!(
                        "Sent {} to {}",
                        file_path.file_name().unwrap().to_str().unwrap(),
                        remote
                    );
                }
                Err(err) => {
                    eprintln!("{err}");
                }
            }
        });
    }

    /// Receives a file from the remote [`SocketAddr`] that has sent a write request using
    /// a random port, asynchronously.
    pub fn receive(
        addr: SocketAddr,
        remote: SocketAddr,
        file_path: PathBuf,
        mut options: Vec<TransferOption>,
    ) {
        thread::spawn(move || {
            let mut handle_receive = || -> Result<(), Box<dyn Error>> {
                let socket = setup_socket(&addr, &remote)?;
                let work_type = WorkType::Receive;
                let worker_options = parse_options(&mut options, &work_type)?;

                accept_request(&socket, &options, &work_type)?;
                receive_file(&socket, &mut File::create(&file_path)?, &worker_options)?;

                Ok(())
            };

            match handle_receive() {
                Ok(_) => {
                    println!(
                        "Received {} from {}",
                        file_path.file_name().unwrap().to_str().unwrap(),
                        remote
                    );
                }
                Err(err) => {
                    eprintln!("{err}");
                    if fs::remove_file(&file_path).is_err() {
                        eprintln!(
                            "Error while cleaning {}",
                            file_path.file_name().unwrap().to_str().unwrap()
                        );
                    }
                }
            }
        });
    }
}

fn send_file(
    socket: &UdpSocket,
    file: File,
    worker_options: &WorkerOptions,
) -> Result<(), Box<dyn Error>> {
    let mut block_number = 1;
    let mut window = Window::new(worker_options.windowsize, worker_options.blk_size, file);
    loop {
        let filled = window.fill()?;

        let mut retry_cnt = 0;
        let mut time = SystemTime::now() - Duration::from_secs(DEFAULT_TIMEOUT_SECS);
        loop {
            if time.elapsed()? >= Duration::from_secs(DEFAULT_TIMEOUT_SECS) {
                send_window(socket, &window, block_number)?;
                time = SystemTime::now();
            }

            match Message::recv(socket) {
                Ok(Packet::Ack(received_block_number)) => {
                    let diff = received_block_number.wrapping_sub(block_number);
                    if diff <= worker_options.windowsize {
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
                        return Err(format!("Transfer timed out after {MAX_RETRIES} tries").into());
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

fn receive_file(
    socket: &UdpSocket,
    file: &mut File,
    worker_options: &WorkerOptions,
) -> Result<(), Box<dyn Error>> {
    let mut block_number: u16 = 0;
    loop {
        let size;

        let mut retry_cnt = 0;
        loop {
            match Message::recv_packet_with_size(socket, worker_options.blk_size) {
                Ok(Packet::Data {
                    block_num: received_block_number,
                    data,
                }) => {
                    if received_block_number == block_number.wrapping_add(1) {
                        block_number = received_block_number;
                        file.write_all(&data)?;
                        size = data.len();
                        break;
                    }
                }
                Ok(Packet::Error { code, msg }) => {
                    return Err(format!("Received error code {code}: {msg}").into());
                }
                _ => {
                    retry_cnt += 1;
                    if retry_cnt == MAX_RETRIES {
                        return Err(format!("Transfer timed out after {MAX_RETRIES} tries").into());
                    }
                }
            }
        }

        Message::send_ack(socket, block_number)?;
        if size < worker_options.blk_size {
            break;
        };
    }

    Ok(())
}

fn send_window(
    socket: &UdpSocket,
    window: &Window,
    mut block_num: u16,
) -> Result<(), Box<dyn Error>> {
    for frame in window.get_elements() {
        Message::send_data(socket, block_num, frame.to_vec())?;
        block_num = block_num.wrapping_add(1);
    }

    Ok(())
}

fn accept_request(
    socket: &UdpSocket,
    options: &Vec<TransferOption>,
    work_type: &WorkType,
) -> Result<(), Box<dyn Error>> {
    if !options.is_empty() {
        Message::send_oack(socket, options.to_vec())?;
        if let WorkType::Send(_) = work_type {
            check_response(socket)?;
        }
    } else if *work_type == WorkType::Receive {
        Message::send_ack(socket, 0)?
    }

    Ok(())
}

fn check_response(socket: &UdpSocket) -> Result<(), Box<dyn Error>> {
    if let Packet::Ack(received_block_number) = Message::recv(socket)? {
        if received_block_number != 0 {
            Message::send_error(socket, ErrorCode::IllegalOperation, "invalid oack response")?;
        }
    }

    Ok(())
}

fn setup_socket(addr: &SocketAddr, remote: &SocketAddr) -> Result<UdpSocket, Box<dyn Error>> {
    let socket = UdpSocket::bind(SocketAddr::from((addr.ip(), 0)))?;
    socket.connect(remote)?;
    socket.set_read_timeout(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)))?;
    socket.set_write_timeout(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)))?;
    Ok(socket)
}

fn parse_options(
    options: &mut Vec<TransferOption>,
    work_type: &WorkType,
) -> Result<WorkerOptions, Box<dyn Error>> {
    let mut worker_options = WorkerOptions {
        blk_size: DEFAULT_BLOCK_SIZE,
        t_size: 0,
        timeout: DEFAULT_TIMEOUT_SECS,
        windowsize: 1,
    };

    for option in &mut *options {
        let TransferOption { option, value } = option;

        match option {
            OptionType::BlockSize => worker_options.blk_size = *value,
            OptionType::TransferSize => match work_type {
                WorkType::Send(size) => {
                    *value = *size as usize;
                    worker_options.t_size = *size as usize;
                }
                WorkType::Receive => {
                    worker_options.t_size = *value;
                }
            },
            OptionType::Timeout => {
                if *value == 0 {
                    return Err("Invalid timeout value".into());
                }
                worker_options.timeout = *value as u64;
            }
            OptionType::Windowsize => {
                if *value == 0 || *value > u16::MAX as usize {
                    return Err("Invalid windowsize value".into());
                }
                worker_options.windowsize = *value as u16;
            }
        }
    }

    Ok(worker_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_send_options() {
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

        let work_type = WorkType::Send(12341234);

        let worker_options = parse_options(&mut options, &work_type).unwrap();

        assert_eq!(options[0].value, worker_options.blk_size);
        assert_eq!(options[1].value, worker_options.t_size);
        assert_eq!(options[2].value as u64, worker_options.timeout);
    }

    #[test]
    fn parses_receive_options() {
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

        let work_type = WorkType::Receive;

        let worker_options = parse_options(&mut options, &work_type).unwrap();

        assert_eq!(options[0].value, worker_options.blk_size);
        assert_eq!(options[1].value, worker_options.t_size);
        assert_eq!(options[2].value as u64, worker_options.timeout);
    }

    #[test]
    fn parses_default_options() {
        assert_eq!(
            parse_options(&mut vec![], &WorkType::Receive).unwrap(),
            WorkerOptions {
                blk_size: DEFAULT_BLOCK_SIZE,
                t_size: 0,
                timeout: DEFAULT_TIMEOUT_SECS,
                windowsize: 1,
            }
        );
    }
}
