use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, UdpSocket},
    path::Path,
    thread,
    time::Duration,
};

use crate::{
    packet::{ErrorCode, OptionType, Packet, TransferOption},
    Message,
};

pub struct Worker;

pub struct WorkerOptions {
    blk_size: usize,
    t_size: usize,
    timeout: u64,
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
    pub fn send(
        addr: SocketAddr,
        remote: SocketAddr,
        filename: String,
        mut options: Vec<TransferOption>,
    ) {
        thread::spawn(move || {
            let mut handle_send = || -> Result<(), Box<dyn Error>> {
                let socket = setup_socket(&addr, &remote)?;
                let work_type = WorkType::Send(Path::new(&filename).metadata().unwrap().len());
                let worker_options = parse_options(&mut options, &work_type)?;
                send_oack(&socket, &options, &work_type)?;
                send_file(&socket, &worker_options, &filename, &mut options)?;

                Ok(())
            };

            if let Err(err) = handle_send() {
                eprintln!("{err}");
            }
        });
    }

    pub fn receive(
        addr: SocketAddr,
        remote: SocketAddr,
        filename: String,
        mut options: Vec<TransferOption>,
    ) {
        thread::spawn(move || {
            let mut handle_receive = || -> Result<(), Box<dyn Error>> {
                let socket = setup_socket(&addr, &remote)?;
                let work_type = WorkType::Receive;
                let worker_options = parse_options(&mut options, &work_type)?;
                send_oack(&socket, &options, &work_type)?;
                receive_file(&socket, &worker_options, &filename, &mut options)?;

                Ok(())
            };

            if let Err(err) = handle_receive() {
                eprintln!("{err}");
            }
        });
    }
}

fn send_file(
    socket: &UdpSocket,
    worker_options: &WorkerOptions,
    filename: &String,
    options: &mut Vec<TransferOption>,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(filename).unwrap();

    parse_options(options, &WorkType::Send(file.metadata().unwrap().len()))?;

    let mut block_number = 1;
    loop {
        let mut chunk = vec![0; worker_options.blk_size];
        let size = file.read(&mut chunk)?;

        let mut retry_cnt = 0;
        loop {
            Message::send_data(socket, block_number, &chunk[..size])?;

            match Message::recv(socket) {
                Ok(Packet::Ack(received_block_number)) => {
                    if received_block_number == block_number {
                        block_number = block_number.wrapping_add(1);
                        break;
                    }
                }
                Ok(Packet::Error { code, msg }) => {
                    return Err(format!("received error code {code}, with message {msg}").into());
                }
                _ => {
                    retry_cnt += 1;
                    if retry_cnt == MAX_RETRIES {
                        return Err(format!("transfer timed out after {MAX_RETRIES} tries").into());
                    }
                }
            }
        }

        if size < worker_options.blk_size {
            break;
        };
    }

    println!("Sent {filename} to {}", socket.peer_addr().unwrap());
    Ok(())
}

fn receive_file(
    socket: &UdpSocket,
    worker_options: &WorkerOptions,
    filename: &String,
    options: &mut Vec<TransferOption>,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(filename).unwrap();

    parse_options(options, &WorkType::Receive)?;

    let mut block_number: u16 = 0;
    loop {
        let size;

        let mut retry_cnt = 0;
        loop {
            match Message::recv_data(socket, worker_options.blk_size) {
                Ok(Packet::Data {
                    block_num: received_block_number,
                    data,
                }) => {
                    if received_block_number == block_number.wrapping_add(1) {
                        block_number = received_block_number;
                        file.write(&data)?;
                        size = data.len();
                        break;
                    }
                }
                Ok(Packet::Error { code, msg }) => {
                    return Err(format!("received error code {code}: {msg}").into());
                }
                Err(err) => {
                    retry_cnt += 1;
                    if retry_cnt == MAX_RETRIES {
                        return Err(
                            format!("transfer timed out after {MAX_RETRIES} tries: {err}").into(),
                        );
                    }
                }
                _ => {}
            }
        }

        Message::send_ack(socket, block_number)?;
        if size < worker_options.blk_size {
            break;
        };
    }

    println!("Received {filename} from {}", socket.peer_addr().unwrap());
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
    };

    for option in &mut *options {
        let TransferOption { option, value } = option;

        match option {
            OptionType::BlockSize => worker_options.blk_size = *value,
            OptionType::TransferSize => match work_type {
                WorkType::Send(size) => {
                    *value = *size as usize;
                }
                WorkType::Receive => {
                    worker_options.t_size = *value;
                }
            },
            OptionType::Timeout => {
                if *value == 0 {
                    return Err("invalid timeout value".into());
                }
                worker_options.timeout = *value as u64;
            }
        }
    }

    Ok(worker_options)
}

fn send_oack(
    socket: &UdpSocket,
    options: &Vec<TransferOption>,
    work_type: &WorkType,
) -> Result<(), Box<dyn Error>> {
    if options.len() > 0 {
        Message::send_oack(socket, options)?;
        if let Packet::Ack(received_block_number) = Message::recv(socket)? {
            if received_block_number != 0 {
                Message::send_error(socket, ErrorCode::IllegalOperation, "invalid oack response")?;
            }
        }
    } else if *work_type == WorkType::Receive {
        Message::send_ack(socket, 0)?
    }

    Ok(())
}
