use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

use crate::packet::{ErrorCode, Opcode, Packet, TransferOption};

pub struct Message;

const MAX_REQUEST_PACKET_SIZE: usize = 512;

impl Message {
    pub fn send_data(
        socket: &UdpSocket,
        block_number: u16,
        data: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        let buf = [
            &Opcode::Data.as_bytes()[..],
            &block_number.to_be_bytes(),
            data,
        ]
        .concat();

        socket.send(&buf)?;

        Ok(())
    }

    pub fn send_ack(socket: &UdpSocket, block_number: u16) -> Result<(), Box<dyn Error>> {
        let buf = [Opcode::Ack.as_bytes(), block_number.to_be_bytes()].concat();

        socket.send(&buf)?;

        Ok(())
    }

    pub fn send_error(
        socket: &UdpSocket,
        code: ErrorCode,
        msg: &str,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&build_error_buf(code, msg))?;

        Ok(())
    }

    pub fn send_error_to<'a>(
        socket: &UdpSocket,
        to: &SocketAddr,
        code: ErrorCode,
        msg: &'a str,
    ) -> Result<(), Box<dyn Error>> {
        if socket.send_to(&build_error_buf(code, msg), to).is_err() {
            eprintln!("could not send an error message");
        }
        Err(msg.into())
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

    pub fn recv(socket: &UdpSocket) -> Result<Packet, Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        let number_of_bytes = socket.recv(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok(packet)
    }

    pub fn recv_data(socket: &UdpSocket, size: usize) -> Result<Packet, Box<dyn Error>> {
        let mut buf = vec![0; size + 4];
        let number_of_bytes = socket.recv(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok(packet)
    }

    pub fn recv_from(socket: &UdpSocket) -> Result<(Packet, SocketAddr), Box<dyn Error>> {
        let mut buf = [0; MAX_REQUEST_PACKET_SIZE];
        let (number_of_bytes, from) = socket.recv_from(&mut buf)?;
        let packet = Packet::deserialize(&buf[..number_of_bytes])?;

        Ok((packet, from))
    }
}

fn build_error_buf(code: ErrorCode, msg: &str) -> Vec<u8> {
    [
        &Opcode::Error.as_bytes()[..],
        &code.as_bytes()[..],
        &msg.as_bytes()[..],
        &[0x00],
    ]
    .concat()
}
