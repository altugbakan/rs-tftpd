#![warn(missing_docs)]

//! Multithreaded TFTP daemon implemented in pure Rust.
//!
//! This server implements [RFC 1350](https://www.rfc-editor.org/rfc/rfc1350), The TFTP Protocol (Revision 2).
//! It also supports the following [RFC 2347](https://www.rfc-editor.org/rfc/rfc2347) TFTP Option Extensions:
//!
//! - [RFC 2348](https://www.rfc-editor.org/rfc/rfc2348) Blocksize Option
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Timeout Interval Option
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Transfer Size Option
//! - [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize Option
//!
//! # Security
//!
//! Since TFTP servers do not offer any type of login or access control mechanisms, this server only allows
//! transfer and receiving inside a chosen folder, and disallows external file access.

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "client")]
mod client_config;
mod config;
mod convert;
mod log;
mod options;
mod packet;
mod server;
mod socket;
mod window;
mod worker;

#[cfg(feature = "debug_drop")]
mod drop;

#[cfg(feature = "client")]
pub use client::Client;
#[cfg(feature = "client")]
pub use client::Mode;
#[cfg(feature = "client")]
pub use client_config::ClientConfig;
pub use config::Config;
pub use convert::Convert;
pub use log::verbosity;
pub use options::OptionType;
pub use options::TransferOption;
pub use packet::ErrorCode;
pub use packet::Opcode;
pub use packet::Packet;
pub use server::Server;
pub use socket::ServerSocket;
pub use socket::Socket;
pub use window::WindowRead;
pub use window::WindowWrite;
pub use worker::Worker;
