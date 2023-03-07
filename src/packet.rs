use crate::Convert;
use std::error::Error;

pub enum Packet<'a> {
    Rrq {
        filename: String,
        mode: String,
        options: Vec<Option>,
    },
    Wrq {
        filename: String,
        mode: String,
        options: Vec<Option>,
    },
    Data {
        block_num: u16,
        data: &'a [u8],
    },
    Ack(u16),
    Error {
        code: ErrorCode,
        msg: String,
    },
}

impl<'a> Packet<'a> {
    pub fn deserialize(buf: &'a [u8]) -> Result<Packet, Box<dyn Error>> {
        let opcode = Opcode::from_u16(Convert::to_u16(&buf[0..1])?)?;

        match opcode {
            Opcode::Rrq | Opcode::Wrq => parse_rq(buf, opcode),
            Opcode::Data => parse_data(buf),
            Opcode::Ack => parse_ack(buf),
            Opcode::Error => parse_error(buf),
        }
    }
}

pub enum Opcode {
    Rrq = 0x0001,
    Wrq = 0x0002,
    Data = 0x0003,
    Ack = 0x0004,
    Error = 0x0005,
}

impl Opcode {
    pub fn from_u16(val: u16) -> Result<Opcode, &'static str> {
        match val {
            0x0001 => Ok(Opcode::Rrq),
            0x0002 => Ok(Opcode::Wrq),
            0x0003 => Ok(Opcode::Data),
            0x0004 => Ok(Opcode::Ack),
            0x0005 => Ok(Opcode::Error),
            _ => Err("invalid opcode"),
        }
    }
}

pub struct Option {
    option: String,
    value: String,
}

#[repr(u16)]
#[derive(PartialEq)]
pub enum ErrorCode {
    NotDefined = 0,
    FileNotFound = 1,
    AccessViolation = 2,
    DiskFull = 3,
    IllegalOperation = 4,
    UnknownId = 5,
    FileExists = 6,
    NoSuchUser = 7,
}

impl ErrorCode {
    pub fn from_u16(code: u16) -> Result<ErrorCode, &'static str> {
        match code {
            0 => Ok(ErrorCode::NotDefined),
            1 => Ok(ErrorCode::FileNotFound),
            2 => Ok(ErrorCode::AccessViolation),
            3 => Ok(ErrorCode::DiskFull),
            4 => Ok(ErrorCode::IllegalOperation),
            5 => Ok(ErrorCode::UnknownId),
            6 => Ok(ErrorCode::FileExists),
            7 => Ok(ErrorCode::NoSuchUser),
            _ => Err("invalid error code"),
        }
    }
}

fn parse_rq(buf: &[u8], opcode: Opcode) -> Result<Packet, Box<dyn Error>> {
    let mut options = vec![];
    let filename: String;
    let mode: String;
    let mut zero_index: usize;

    (filename, zero_index) = Convert::to_string(buf, 2)?;
    (mode, zero_index) = Convert::to_string(buf, zero_index + 1)?;

    let mut value: String;
    let mut option;
    while zero_index < buf.len() - 1 {
        (option, zero_index) = Convert::to_string(buf, zero_index + 1)?;
        (value, zero_index) = Convert::to_string(buf, zero_index + 1)?;
        options.push(Option { option, value });
    }

    match opcode {
        Opcode::Rrq => Ok(Packet::Rrq {
            filename,
            mode,
            options,
        }),
        Opcode::Wrq => Ok(Packet::Wrq {
            filename,
            mode,
            options,
        }),
        _ => Err("non request opcode".into()),
    }
}

fn parse_data(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    Ok(Packet::Data {
        block_num: Convert::to_u16(&buf[2..])?,
        data: &buf[2..],
    })
}

fn parse_ack(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    Ok(Packet::Ack(Convert::to_u16(&buf[2..])?))
}

fn parse_error(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    let code = ErrorCode::from_u16(Convert::to_u16(&buf[2..])?)?;
    let (msg, _) = Convert::to_string(buf, 4)?;
    Ok(Packet::Error { code, msg })
}
