use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
    path::Path,
};

use crate::packet::Option;

pub struct Worker {
    socket: UdpSocket,
}

impl Worker {
    pub fn new(addr: SocketAddr, remote: SocketAddr) -> Result<Worker, Box<dyn Error>> {
        let socket = UdpSocket::bind(SocketAddr::from((addr.ip(), 0)))?;
        socket.connect(remote)?;
        Ok(Worker { socket })
    }

    pub fn send_file(&self, file: &Path, options: &Vec<Option>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub fn receive_file(&self, file: &Path, options: &Vec<Option>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
