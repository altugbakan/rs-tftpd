use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

use crate::packet::{ErrorCode, Opcode, Option};

pub struct Message;

impl Message {
    pub fn send_data(socket: &UdpSocket, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let buf = [&[0x00, Opcode::Data as u8], data].concat();

        socket.send(&buf)?;

        Ok(())
    }

    pub fn send_ack(socket: &UdpSocket, block: u16) -> Result<(), Box<dyn Error>> {
        let buf = [&[0x00, Opcode::Ack as u8], &block.to_be_bytes()[..]].concat();

        socket.send(&buf)?;

        Ok(())
    }

    pub fn send_error(
        socket: &UdpSocket,
        code: ErrorCode,
        msg: &str,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&get_error_buf(code, msg))?;

        Ok(())
    }

    pub fn send_error_to(socket: &UdpSocket, to: &SocketAddr, code: ErrorCode, msg: &str) {
        if socket.send_to(&get_error_buf(code, msg), to).is_err() {
            eprintln!("could not send an error message");
        }
    }

    pub fn send_oack_to(
        socket: &UdpSocket,
        to: &SocketAddr,
        options: Vec<Option>,
    ) -> Result<(), Box<dyn Error>> {
        todo!();

        let buf = [];

        socket.send_to(&buf, to)?;

        Ok(())
    }
}

fn get_error_buf(code: ErrorCode, msg: &str) -> Vec<u8> {
    [
        &[0x00, Opcode::Error as u8, 0x00, code as u8],
        msg.as_bytes(),
        &[0x00],
    ]
    .concat()
}
