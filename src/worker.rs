use std::{
    error::Error,
    fs::File,
    io::Read,
    net::{SocketAddr, UdpSocket},
    path::Path,
    time::Duration,
};

use crate::{
    packet::{OptionType, TransferOption},
    Message,
};

pub struct Worker {
    socket: UdpSocket,
    blk_size: usize,
    t_size: usize,
    timeout: usize,
}

const MAX_RETRIES: u32 = 6;

impl Worker {
    pub fn new(addr: &SocketAddr, remote: &SocketAddr) -> Result<Worker, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((addr.ip(), 0)))?;
        socket.connect(remote)?;
        Ok(Worker {
            socket,
            blk_size: 512,
            t_size: 0,
            timeout: 5,
        })
    }

    pub fn send_file(
        &mut self,
        file: &Path,
        options: &Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file).unwrap();

        self.parse_options(options, Some(&file));
        Message::send_oack(&self.socket, options)?;

        self.socket
            .set_write_timeout(Some(Duration::from_secs(self.timeout as u64)))?;

        let mut retry_cnt = 0;
        loop {
            let mut chunk = Vec::with_capacity(self.blk_size);
            let size = file
                .by_ref()
                .take(self.blk_size as u64)
                .read_to_end(&mut chunk)?;

            loop {
                if Message::send_data(&self.socket, &chunk).is_err() {
                    return Err(format!("failed to send data").into());
                }

                if let Ok(block) = Message::receive_ack(&self.socket) {
                    todo!("handle block number");
                } else {
                    retry_cnt += 1;
                    if retry_cnt == MAX_RETRIES {
                        return Err(format!("transfer timed out after {MAX_RETRIES} tries").into());
                    }
                }
            }

            if size < self.blk_size {
                break;
            };
        }

        Ok(())
    }

    pub fn receive_file(
        &mut self,
        file: &Path,
        options: &Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file).unwrap();

        self.parse_options(options, Some(&file));
        Message::send_oack(&self.socket, options)?;

        todo!("file receiving");

        Ok(())
    }

    fn parse_options(&mut self, options: &Vec<TransferOption>, file: Option<&File>) {
        for option in options {
            let TransferOption { option, value } = option;

            match option {
                OptionType::BlockSize => self.blk_size = *value,
                OptionType::TransferSize => {
                    self.t_size = match file {
                        Some(file) => file.metadata().unwrap().len() as usize,
                        None => *value,
                    }
                }
                OptionType::Timeout => self.timeout = *value,
            }
        }
    }
}
