use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

use crate::packet::{ErrorCode, Packet, TransferOption};

pub struct Message;

const MAX_REQUEST_PACKET_SIZE: usize = 512;

impl Message {
    pub fn send_data(
        socket: &UdpSocket,
        block_num: u16,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Data { block_num, data }.serialize()?)?;

        Ok(())
    }

    pub fn send_ack(socket: &UdpSocket, block_number: u16) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Ack(block_number).serialize()?)?;

        Ok(())
    }

    pub fn send_error(
        socket: &UdpSocket,
        code: ErrorCode,
        msg: String,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Error { code, msg }.serialize()?)?;

        Ok(())
    }

    pub fn send_error_to<'a>(
        socket: &UdpSocket,
        to: &SocketAddr,
        code: ErrorCode,
        msg: String,
    ) -> Result<(), Box<dyn Error>> {
        if socket
            .send_to(
                &Packet::Error {
                    code,
                    msg: msg.clone(),
                }
                .serialize()?,
                to,
            )
            .is_err()
        {
            eprintln!("could not send an error message");
        }
        Err(msg.into())
    }

    pub fn send_oack(
        socket: &UdpSocket,
        options: Vec<TransferOption>,
    ) -> Result<(), Box<dyn Error>> {
        socket.send(&Packet::Oack { options }.serialize()?)?;

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
