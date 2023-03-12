#![warn(missing_docs)]

//! Multithreaded TFTP daemon implemented in pure Rust without dependencies.
//!
//! This server implements [RFC 1350](https://www.rfc-editor.org/rfc/rfc1350), The TFTP Protocol (Revision 2).
//! It also supports the following [RFC 2347](https://www.rfc-editor.org/rfc/rfc2347) TFTP Option Extensions:
//!
//! - [RFC 2348](https://www.rfc-editor.org/rfc/rfc2348) Blocksize Option
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Timeout Interval Option
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Transfer Size Option
//!
//! # Security
//!
//! Since TFTP servers do not offer any type of login or access control mechanisms, this server only allows
//! transfer and receiving inside a chosen folder, and disallows external file access.

mod config;
mod convert;
mod message;
mod packet;
mod server;
mod worker;

pub use config::Config;
pub use convert::Convert;
pub use message::Message;
pub use packet::ErrorCode;
pub use packet::Opcode;
pub use packet::OptionType;
pub use packet::Packet;
pub use packet::TransferOption;
pub use server::Server;
pub use worker::Worker;
