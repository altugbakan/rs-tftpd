use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

use crate::packet::{ErrorCode, Opcode, Packet, TransferOption};

pub struct Message;

impl Message {
    pub fn send_data(socket: &UdpSocket, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let buf = [&Opcode::Data.as_bytes()[..], data].concat();

        socket.send(&buf)?;

        Ok(())
    }

    pub fn send_ack(socket: &UdpSocket, block: u16) -> Result<(), Box<dyn Error>> {
        let buf = [Opcode::Ack.as_bytes(), block.to_be_bytes()].concat();

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
        eprintln!("{msg}");
        if socket.send_to(&get_error_buf(code, msg), to).is_err() {
            eprintln!("could not send an error message");
        }
    }

    pub fn send_oack(
        socket: &UdpSocket,
        options: &Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        let mut buf = Opcode::Oack.as_bytes().to_vec();

        for option in options {
            buf = [buf, option.as_bytes()].concat();
        }

        socket.send(&buf)?;

        Ok(())
    }

    pub fn receive_ack(socket: &UdpSocket) -> Result<u16, Box<dyn Error>> {
        let mut buf = [0; 4];
        socket.recv(&mut buf)?;

        if let Ok(Packet::Ack(block)) = Packet::deserialize(&buf) {
            Ok(block)
        } else {
            Err("invalid ack".into())
        }
    }
}

fn get_error_buf(code: ErrorCode, msg: &str) -> Vec<u8> {
    [
        &Opcode::Error.as_bytes()[..],
        &code.as_bytes()[..],
        &msg.as_bytes()[..],
        &[0x00],
    ]
    .concat()
}
