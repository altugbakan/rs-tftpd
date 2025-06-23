use std::error::Error;
use std::str::FromStr;
use std::fmt;

use crate::{Convert, TransferOption, OptionType};

/// Packet `enum` represents the valid TFTP packet types.
///
/// This `enum` has function implementaions for serializing [`Packet`]s into
///  [`Vec<u8>`]s and deserializing [`u8`] slices to [`Packet`]s.
///
/// # Example
/// ```rust
/// use tftpd::Packet;
///
/// let packet = Packet::Data { block_num: 15, data: vec![0x01, 0x02, 0x03] };
///
/// assert_eq!(packet.serialize().unwrap(), vec![0x00, 0x03, 0x00, 0x0F, 0x01, 0x02, 0x03]);
/// assert_eq!(Packet::deserialize(&[0x00, 0x03, 0x00, 0x0F, 0x01, 0x02, 0x03]).unwrap(), packet);
/// ```
#[derive(Debug, PartialEq)]
pub enum Packet {
    /// Read Request `struct`
    Rrq {
        /// Name of the requested file
        filename: String,
        /// Transfer mode
        mode: String,
        /// Transfer options
        options: Vec<TransferOption>,
    },
    /// Write Request `struct`
    Wrq {
        /// Name of the requested file
        filename: String,
        /// Transfer mode
        mode: String,
        /// Transfer options
        options: Vec<TransferOption>,
    },
    /// Data `struct`
    Data {
        /// Block number
        block_num: u16,
        /// Data
        data: Vec<u8>,
    },
    /// Acknowledgement `tuple` with block number
    Ack(u16),
    /// Error `struct`
    Error {
        /// Error code
        code: ErrorCode,
        /// Error message
        msg: String,
    },
    /// Option acknowledgement `tuple` with transfer options
    Oack(Vec<TransferOption>),
}

impl Packet {
    /// Deserializes a [`u8`] slice into a [`Packet`].
    pub fn deserialize(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
        if buf.len() < 2 {
            return Err("Buffer too short to serialize".into());
        }
        let opcode = Opcode::from_u16(Convert::to_u16(&buf[0..=1])?)?;

        match opcode {
            Opcode::Rrq | Opcode::Wrq => parse_rq(buf, opcode),
            Opcode::Data => parse_data(buf),
            Opcode::Ack => parse_ack(buf),
            Opcode::Oack => parse_oack(buf),
            Opcode::Error => parse_error(buf),
        }
    }

    /// Serializes a [`Packet`] into a [`Vec<u8>`].
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        match self {
            Packet::Rrq {
                filename,
                mode,
                options,
            } => Ok(serialize_rrq(filename, mode, options)),
            Packet::Wrq {
                filename,
                mode,
                options,
            } => Ok(serialize_wrq(filename, mode, options)),
            Packet::Data { block_num, data } => Ok(serialize_data(block_num, data)),
            Packet::Ack(block_num) => Ok(serialize_ack(block_num)),
            Packet::Error { code, msg } => Ok(serialize_error(code, msg)),
            Packet::Oack(options) => Ok(serialize_oack(options)),
        }
    }
}

/// Opcode `enum` represents the opcodes used in the TFTP definition.
///
/// This `enum` has function implementations for converting [`u16`]s to
/// [`Opcode`]s and [`Opcode`]s to [`u8`] arrays.
///
/// # Example
///
/// ```rust
/// use tftpd::Opcode;
///
/// assert_eq!(Opcode::from_u16(3).unwrap(), Opcode::Data);
/// assert_eq!(Opcode::Ack.as_bytes(), [0x00, 0x04]);
/// ```
#[repr(u16)]
#[derive(Debug, PartialEq)]
pub enum Opcode {
    /// Read request opcode
    Rrq = 0x0001,
    /// Write request opcode
    Wrq = 0x0002,
    /// Data opcode
    Data = 0x0003,
    /// Acknowledgement opcode
    Ack = 0x0004,
    /// Error opcode
    Error = 0x0005,
    /// Option acknowledgement opcode
    Oack = 0x0006,
}

impl Opcode {
    /// Converts a [`u16`] to an [`Opcode`].
    pub fn from_u16(val: u16) -> Result<Opcode, &'static str> {
        match val {
            0x0001 => Ok(Opcode::Rrq),
            0x0002 => Ok(Opcode::Wrq),
            0x0003 => Ok(Opcode::Data),
            0x0004 => Ok(Opcode::Ack),
            0x0005 => Ok(Opcode::Error),
            0x0006 => Ok(Opcode::Oack),
            _ => Err("Invalid opcode"),
        }
    }

    /// Converts a [`u16`] to a [`u8`] array with 2 elements.
    pub const fn as_bytes(self) -> [u8; 2] {
        (self as u16).to_be_bytes()
    }
}

/// ErrorCode `enum` represents the error codes used in the TFTP definition.
///
/// This `enum` has function implementations for converting [`u16`]s to
/// [`ErrorCode`]s and [`ErrorCode`]s to [`u8`] arrays.
///
/// # Example
///
/// ```rust
/// use tftpd::ErrorCode;
///
/// assert_eq!(ErrorCode::from_u16(3).unwrap(), ErrorCode::DiskFull);
/// assert_eq!(ErrorCode::FileExists.as_bytes(), [0x00, 0x06]);
/// ```
#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ErrorCode {
    /// Not Defined error code
    NotDefined = 0,
    /// File not found error code
    FileNotFound = 1,
    /// Access violation error code
    AccessViolation = 2,
    /// Disk full error code
    DiskFull = 3,
    /// Illegal operation error code
    IllegalOperation = 4,
    /// Unknown ID error code
    UnknownId = 5,
    /// File exists error code
    FileExists = 6,
    /// No such user error code
    NoSuchUser = 7,
    /// Refused option error code
    RefusedOption = 8,
}

impl ErrorCode {
    /// Converts a [`u16`] to an [`ErrorCode`].
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
            8 => Ok(ErrorCode::RefusedOption),
            _ => Err("Invalid error code"),
        }
    }

    /// Converts an [`ErrorCode`] to a [`u8`] array with 2 elements.
    pub fn as_bytes(self) -> [u8; 2] {
        (self as u16).to_be_bytes()
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::NotDefined => write!(f, "Not Defined"),
            ErrorCode::FileNotFound => write!(f, "File Not Found"),
            ErrorCode::AccessViolation => write!(f, "Access Violation"),
            ErrorCode::DiskFull => write!(f, "Disk Full"),
            ErrorCode::IllegalOperation => write!(f, "Illegal Operation"),
            ErrorCode::UnknownId => write!(f, "Unknown ID"),
            ErrorCode::FileExists => write!(f, "File Exists"),
            ErrorCode::NoSuchUser => write!(f, "No Such User"),
            ErrorCode::RefusedOption => write!(f, "Refused option"),
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

        if let Ok(option) = OptionType::from_str(option.to_lowercase().as_str()) {
            options.push(TransferOption {
                option,
                value: value.parse()?,
            });
        }
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
        _ => Err("Non request opcode".into()),
    }
}

fn parse_data(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    Ok(Packet::Data {
        block_num: Convert::to_u16(&buf[2..])?,
        data: buf[4..].to_vec(),
    })
}

fn parse_ack(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    Ok(Packet::Ack(Convert::to_u16(&buf[2..])?))
}

fn parse_oack(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    let mut options = vec![];
    let mut value: String;
    let mut option;
    let mut zero_index = 1usize;

    while zero_index < buf.len() - 1 {
        (option, zero_index) = Convert::to_string(buf, zero_index + 1)?;
        (value, zero_index) = Convert::to_string(buf, zero_index + 1)?;
        if let Ok(option) = OptionType::from_str(option.to_lowercase().as_str()) {
            options.push(TransferOption {
                option,
                value: value.parse()?,
            });
        }
    }

    Ok(Packet::Oack(options))
}

fn parse_error(buf: &[u8]) -> Result<Packet, Box<dyn Error>> {
    let code = ErrorCode::from_u16(Convert::to_u16(&buf[2..])?)?;
    if let Ok((msg, _)) = Convert::to_string(buf, 4) {
        Ok(Packet::Error { code, msg })
    } else {
        Ok(Packet::Error {
            code,
            msg: "(no message)".to_string(),
        })
    }
}

fn serialize_rrq(filename: &String, mode: &String, options: &Vec<TransferOption>) -> Vec<u8> {
    let mut buf = [
        &Opcode::Rrq.as_bytes(),
        filename.as_bytes(),
        &[0x00],
        mode.as_bytes(),
        &[0x00],
    ]
    .concat();

    for option in options {
        buf = [buf, option.as_bytes()].concat();
    }
    buf
}

fn serialize_wrq(filename: &String, mode: &String, options: &Vec<TransferOption>) -> Vec<u8> {
    let mut buf = [
        &Opcode::Wrq.as_bytes(),
        filename.as_bytes(),
        &[0x00],
        mode.as_bytes(),
        &[0x00],
    ]
    .concat();

    for option in options {
        buf = [buf, option.as_bytes()].concat();
    }
    buf
}

fn serialize_data(block_num: &u16, data: &Vec<u8>) -> Vec<u8> {
    [
        &Opcode::Data.as_bytes(),
        &block_num.to_be_bytes(),
        data.as_slice(),
    ]
    .concat()
}

fn serialize_ack(block_num: &u16) -> Vec<u8> {
    [Opcode::Ack.as_bytes(), block_num.to_be_bytes()].concat()
}

fn serialize_error(code: &ErrorCode, msg: &String) -> Vec<u8> {
    [
        &Opcode::Error.as_bytes()[..],
        &code.as_bytes()[..],
        msg.as_bytes(),
        &[0x00],
    ]
    .concat()
}

fn serialize_oack(options: &Vec<TransferOption>) -> Vec<u8> {
    let mut buf = Opcode::Oack.as_bytes().to_vec();

    for option in options {
        buf = [buf, option.as_bytes()].concat();
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_read_request() {
        let buf = [
            &Opcode::Rrq.as_bytes()[..],
            ("test.png".as_bytes()),
            &[0x00],
            ("octet".as_bytes()),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Rrq {
            filename,
            mode,
            options,
        }) = parse_rq(&buf, Opcode::Rrq)
        {
            assert_eq!(filename, "test.png");
            assert_eq!(mode, "octet");
            assert_eq!(options.len(), 0);
        } else {
            panic!("cannot parse read request")
        }
    }

    #[test]
    fn parses_read_request_with_options() {
        let buf = [
            &Opcode::Rrq.as_bytes()[..],
            ("test.png".as_bytes()),
            &[0x00],
            ("octet".as_bytes()),
            &[0x00],
            (OptionType::TransferSize.as_str().as_bytes()),
            &[0x00],
            ("0".as_bytes()),
            &[0x00],
            (OptionType::Timeout.as_str().as_bytes()),
            &[0x00],
            ("5".as_bytes()),
            &[0x00],
            (OptionType::WindowSize.as_str().as_bytes()),
            &[0x00],
            ("4".as_bytes()),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Rrq {
            filename,
            mode,
            options,
        }) = parse_rq(&buf, Opcode::Rrq)
        {
            assert_eq!(filename, "test.png");
            assert_eq!(mode, "octet");
            assert_eq!(options.len(), 3);
            assert_eq!(
                options[0],
                TransferOption {
                    option: OptionType::TransferSize,
                    value: 0
                }
            );
            assert_eq!(
                options[1],
                TransferOption {
                    option: OptionType::Timeout,
                    value: 5
                }
            );
            assert_eq!(
                options[2],
                TransferOption {
                    option: OptionType::WindowSize,
                    value: 4
                }
            );
        } else {
            panic!("cannot parse read request with options")
        }
    }

    #[test]
    fn parses_write_request() {
        let buf = [
            &Opcode::Wrq.as_bytes()[..],
            ("test.png".as_bytes()),
            &[0x00],
            ("octet".as_bytes()),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Wrq {
            filename,
            mode,
            options,
        }) = parse_rq(&buf, Opcode::Wrq)
        {
            assert_eq!(filename, "test.png");
            assert_eq!(mode, "octet");
            assert_eq!(options.len(), 0);
        } else {
            panic!("cannot parse write request")
        }
    }

    #[test]
    fn parses_write_request_with_options() {
        let buf = [
            &Opcode::Wrq.as_bytes()[..],
            ("test.png".as_bytes()),
            &[0x00],
            ("octet".as_bytes()),
            &[0x00],
            (OptionType::TransferSize.as_str().as_bytes()),
            &[0x00],
            ("12341234".as_bytes()),
            &[0x00],
            (OptionType::BlockSize.as_str().as_bytes()),
            &[0x00],
            ("1024".as_bytes()),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Wrq {
            filename,
            mode,
            options,
        }) = parse_rq(&buf, Opcode::Wrq)
        {
            assert_eq!(filename, "test.png");
            assert_eq!(mode, "octet");
            assert_eq!(options.len(), 2);
            assert_eq!(
                options[0],
                TransferOption {
                    option: OptionType::TransferSize,
                    value: 12341234
                }
            );
            assert_eq!(
                options[1],
                TransferOption {
                    option: OptionType::BlockSize,
                    value: 1024
                }
            );
        } else {
            panic!("cannot parse write request with options")
        }
    }

    #[test]
    fn parses_data() {
        let buf = [
            &Opcode::Data.as_bytes()[..],
            &5u16.to_be_bytes(),
            &[
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
            ],
        ]
        .concat();

        if let Ok(Packet::Data { block_num, data }) = parse_data(&buf) {
            assert_eq!(block_num, 5);
            assert_eq!(
                data,
                [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C]
            );
        } else {
            panic!("cannot parse data")
        }
    }

    #[test]
    fn parses_ack() {
        let buf = [&Opcode::Ack.as_bytes()[..], &12u16.to_be_bytes()].concat();

        if let Ok(Packet::Ack(block_num)) = parse_ack(&buf) {
            assert_eq!(block_num, 12);
        } else {
            panic!("cannot parse ack")
        }
    }

    #[test]
    fn parses_oack() {
        let buf = [
            &Opcode::Oack.as_bytes()[..],
            (OptionType::TransferSize.as_str().as_bytes()),
            &[0x00],
            ("0".as_bytes()),
            &[0x00],
            (OptionType::Timeout.as_str().as_bytes()),
            &[0x00],
            ("5".as_bytes()),
            &[0x00],
            (OptionType::WindowSize.as_str().as_bytes()),
            &[0x00],
            ("4".as_bytes()),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Oack(options)) = parse_oack(&buf) {
            assert_eq!(options.len(), 3);
            assert_eq!(
                options[0],
                TransferOption {
                    option: OptionType::TransferSize,
                    value: 0
                }
            );
            assert_eq!(
                options[1],
                TransferOption {
                    option: OptionType::Timeout,
                    value: 5
                }
            );
            assert_eq!(
                options[2],
                TransferOption {
                    option: OptionType::WindowSize,
                    value: 4
                }
            );
        } else {
            panic!("cannot parse read request with options")
        }
    }

    #[test]
    fn parses_error() {
        let buf = [
            &Opcode::Error.as_bytes()[..],
            &ErrorCode::FileExists.as_bytes(),
            "file already exists".as_bytes(),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Error { code, msg }) = parse_error(&buf) {
            assert_eq!(code, ErrorCode::FileExists);
            assert_eq!(msg, "file already exists");
        } else {
            panic!("cannot parse error")
        }
    }

    #[test]
    fn parses_error_without_message() {
        let buf = [
            &Opcode::Error.as_bytes()[..],
            &ErrorCode::FileExists.as_bytes(),
            &[0x00],
        ]
        .concat();

        if let Ok(Packet::Error { code, msg }) = parse_error(&buf) {
            assert_eq!(code, ErrorCode::FileExists);
            assert_eq!(msg, "");
        } else {
            panic!("cannot parse error")
        }
    }

    #[test]
    fn serializes_rrq() {
        let serialized_data = vec![
            0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x6f, 0x63, 0x74, 0x65, 0x74, 0x00,
        ];

        assert_eq!(
            serialize_rrq(&"test".into(), &"octet".into(), &vec![]),
            serialized_data
        )
    }

    #[test]
    fn serializes_rrq_with_options() {
        let serialized_data = vec![
            0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x6f, 0x63, 0x74, 0x65, 0x74, 0x00, 0x62,
            0x6c, 0x6b, 0x73, 0x69, 0x7a, 0x65, 0x00, 0x31, 0x34, 0x36, 0x38, 0x00, 0x77, 0x69,
            0x6e, 0x64, 0x6f, 0x77, 0x73, 0x69, 0x7a, 0x65, 0x00, 0x31, 0x00, 0x74, 0x69, 0x6d,
            0x65, 0x6f, 0x75, 0x74, 0x00, 0x35, 0x00,
        ];

        assert_eq!(
            serialize_rrq(
                &"test".into(),
                &"octet".into(),
                &vec![
                    TransferOption {
                        option: OptionType::BlockSize,
                        value: 1468,
                    },
                    TransferOption {
                        option: OptionType::WindowSize,
                        value: 1,
                    },
                    TransferOption {
                        option: OptionType::Timeout,
                        value: 5,
                    }
                ]
            ),
            serialized_data
        )
    }

    #[test]
    fn serializes_wrq() {
        let serialized_data = vec![
            0x00, 0x02, 0x74, 0x65, 0x73, 0x74, 0x00, 0x6f, 0x63, 0x74, 0x65, 0x74, 0x00,
        ];

        assert_eq!(
            serialize_wrq(&"test".into(), &"octet".into(), &vec![]),
            serialized_data
        )
    }

    #[test]
    fn serializes_wrq_with_options() {
        let serialized_data = vec![
            0x00, 0x02, 0x74, 0x65, 0x73, 0x74, 0x00, 0x6f, 0x63, 0x74, 0x65, 0x74, 0x00, 0x62,
            0x6c, 0x6b, 0x73, 0x69, 0x7a, 0x65, 0x00, 0x31, 0x34, 0x36, 0x38, 0x00, 0x77, 0x69,
            0x6e, 0x64, 0x6f, 0x77, 0x73, 0x69, 0x7a, 0x65, 0x00, 0x31, 0x00, 0x74, 0x69, 0x6d,
            0x65, 0x6f, 0x75, 0x74, 0x00, 0x35, 0x00,
        ];

        assert_eq!(
            serialize_wrq(
                &"test".into(),
                &"octet".into(),
                &vec![
                    TransferOption {
                        option: OptionType::BlockSize,
                        value: 1468,
                    },
                    TransferOption {
                        option: OptionType::WindowSize,
                        value: 1,
                    },
                    TransferOption {
                        option: OptionType::Timeout,
                        value: 5,
                    }
                ]
            ),
            serialized_data
        )
    }

    #[test]
    fn serializes_data() {
        let serialized_data = vec![0x00, 0x03, 0x00, 0x10, 0x01, 0x02, 0x03, 0x04];

        assert_eq!(
            serialize_data(&16, &vec![0x01, 0x02, 0x03, 0x04]),
            serialized_data
        );
    }

    #[test]
    fn serializes_ack() {
        let serialized_ack = vec![0x00, 0x04, 0x04, 0xD2];

        assert_eq!(serialize_ack(&1234), serialized_ack);
    }

    #[test]
    fn serializes_error() {
        let serialized_error = vec![
            0x00, 0x05, 0x00, 0x04, 0x69, 0x6C, 0x6C, 0x65, 0x67, 0x61, 0x6C, 0x20, 0x6F, 0x70,
            0x65, 0x72, 0x61, 0x74, 0x69, 0x6F, 0x6E, 0x00,
        ];

        assert_eq!(
            serialize_error(
                &ErrorCode::IllegalOperation,
                &"illegal operation".to_string()
            ),
            serialized_error
        );
    }

    #[test]
    fn serializes_oack() {
        let serialized_oack = vec![
            0x00, 0x06, 0x62, 0x6C, 0x6B, 0x73, 0x69, 0x7A, 0x65, 0x00, 0x31, 0x34, 0x33, 0x32,
            0x00,
        ];

        assert_eq!(
            serialize_oack(&vec![TransferOption {
                option: OptionType::BlockSize,
                value: 1432
            }]),
            serialized_oack
        );
    }
}
