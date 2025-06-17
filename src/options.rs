use std::error::Error;
use std::time::Duration;
use std::str::FromStr;
use std::fmt;

use crate::{server::RequestType, log::*};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_BLOCK_SIZE: u16 = 512;
pub const DEFAULT_WINDOW_SIZE: u16 = 1;
pub const DEFAULT_WINDOW_WAIT: Duration = Duration::from_millis(0);
pub const DEFAULT_MAX_RETRIES: usize = 6;
pub const DEFAULT_ROLLOVER : Rollover = Rollover::Enforce0;

/// Enum used to set the block counter roll-over policy
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Rollover {
    /// Rollover forbidden
    None,
    /// Enforce 0 in Rx and Tx
    Enforce0,
    /// Enforce 1 in Rx and Tx
    Enforce1,
    /// Allow both cases in Rx and use value in Tx
    DontCare,
}

/// Local options `struct` used for storing and passing options for client and server
/// set directly from executable arguments. Though present on both sides of the
/// transfer, they can differ and are independent.
#[derive(Clone, Debug)]
pub struct OptionsPrivate {
    /// Duplicate all packets sent from the server. (default: 0)
    pub repeat_count: u8,
    /// Should clean (delete) files after receiving errors. (default: true)
    pub clean_on_error: bool,
    /// Max count of retires (default: 6)
    pub max_retries: usize,
    /// Block counter roll-over policy  (default: Enforce0)
    pub rollover: Rollover,
}

impl Default for OptionsPrivate {
    fn default() -> Self {
        Self {
            repeat_count: 1,
            clean_on_error: true,
            max_retries: DEFAULT_MAX_RETRIES,
            rollover: DEFAULT_ROLLOVER,
        }
    }
}

/// Common options `struct` used for storing and passing options for client and server
/// negotiated before data exchange. User can set them on client side as executable
/// arguments, server will then validate and send them back, and client will use this
/// definitive version.
/// Some options are defined by RFC and some others are non standard.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionsProtocol {
    /// Blocksize to use during transfer. (default: 512)
    pub block_size: u16,
    /// Windowsize to use during transfer. (default: 1)
    pub window_size: u16,
    /// Inter packets wait delay in windows (default: 10ms)
    pub window_wait: Duration,
    /// Timeout to use during transfer. (default: 5s)
    pub timeout: Duration,
    /// Size of the file to transfer (default: N/A)
    pub transfer_size: Option<u64>,
}

impl OptionsProtocol {
    pub fn prepare(&self) -> Vec<TransferOption> {
        let mut options = vec![
            TransferOption {
                option: OptionType::BlockSize,
                value: self.block_size as u64,
            },
            TransferOption {
                option: OptionType::TransferSize,
                value: self.transfer_size.unwrap_or(0),
            },
            TransferOption {
                option: OptionType::WindowSize,
                value: self.window_size as u64,
            },
        ];

        if self.window_wait.as_millis() != 0 {
            options.push(TransferOption {
                option: OptionType::WindowWait,
                value: self.window_wait.as_millis() as u64,
            });
        }

        options.push(if self.timeout.subsec_millis() == 0 {
            TransferOption {
                option: OptionType::Timeout,
                value: self.timeout.as_secs(),
            }
        } else {
            TransferOption {
                option: OptionType::TimeoutMs,
                value: self.timeout.as_millis() as u64,
            }
        });

        options
    }

    pub fn parse(options: &mut [TransferOption], request_type: RequestType) -> Result<OptionsProtocol, &'static str> {
        let mut opt_common = OptionsProtocol::default();

        for option in options {
            let TransferOption {
                option: option_type,
                value,
            } = option;

            match option_type {
                OptionType::BlockSize => {
                    if *value == 0  {
                        // RFC 2348 requests block size to be in range 8-65464
                        // but we use 1-65464 as 1 is useful to speed up some tests
                        log_warn!("  Invalid block size 0. Changed to {DEFAULT_BLOCK_SIZE}.");
                        *value = DEFAULT_BLOCK_SIZE as u64;
                    } else if 65464 < *value {
                        log_warn!("  Invalid block size {}. Changed to 65464.", *value);
                        *value = 65464;
                    }
                    opt_common.block_size = *value as u16;
                }
                OptionType::TransferSize => match request_type {
                    RequestType::Read(size) => {
                        *value = size;
                        opt_common.transfer_size = Some(size);
                    }
                    RequestType::Write => opt_common.transfer_size = Some(*value),
                },
                OptionType::Timeout => {
                    if *value == 0  {
                        // RFC 2349 requests timeout to be in range 1-255
                        log_warn!("  Invalid timeout value 0. Changed to 1.");
                        *value = 1;
                    } else if 255 < *value {
                        log_warn!("  Invalid timeout value {}. Changed to 255.", *value);
                        *value = 255;
                    }
                    opt_common.timeout = Duration::from_secs(*value);
                }
                OptionType::TimeoutMs => {
                    if *value == 0  {
                        // RFC 2349 requests timeout to be in range 1-255
                        log_warn!("  Invalid timeoutms value 0. Changed to 1.");
                        *value = 1;
                    } else if 255 < *value {
                        log_warn!("  Invalid timeoutms value {}. Changed to 255.", *value);
                        *value = 255;
                    }
                    opt_common.timeout = Duration::from_millis(*value);
                }
                OptionType::WindowSize => {
                    if *value == 0  {
                        // RFC 7440 requests window to be in range 1-65535
                        log_warn!("  Invalid window size 0. Changed to 1.");
                        *value = 1;
                    } else if 65535 < *value {
                        log_warn!("  Invalid window size {}. Changed to 65535.", *value);
                        *value = 65535;
                    }
                    opt_common.window_size = *value as u16;
                }
                OptionType::WindowWait => {
                    opt_common.window_wait = Duration::from_millis(*value);
                }
            }
        }

        Ok(opt_common)
    }

    pub fn apply(&mut self, options: &Vec<TransferOption>) -> Result<(), Box<dyn Error>> {
        for option in options {
            match option.option {
                OptionType::BlockSize => self.block_size = option.value as u16,
                OptionType::WindowSize => self.window_size = option.value as u16,
                OptionType::WindowWait => self.window_wait = Duration::from_millis(option.value),
                OptionType::Timeout => self.timeout = Duration::from_secs(option.value),
                OptionType::TimeoutMs => self.timeout = Duration::from_millis(option.value),
                OptionType::TransferSize => self.transfer_size = Some(option.value),
            }
        }

        Ok(())
    }
}

impl Default for OptionsProtocol {
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            window_size: DEFAULT_WINDOW_SIZE,
            window_wait: DEFAULT_WINDOW_WAIT,
            timeout: DEFAULT_TIMEOUT,
            transfer_size: None,
        }
    }
}


/// TransferOption `struct` represents the TFTP transfer options.
///
/// This `struct` has a function implementation for converting [`TransferOption`]s
/// to [`Vec<u8>`]s.
///
/// # Example
///
/// ```rust
/// use tftpd::{TransferOption, OptionType};
///
/// assert_eq!(TransferOption { option: OptionType::BlockSize, value: 1432 }.as_bytes(), vec![
///     0x62, 0x6C, 0x6B, 0x73, 0x69, 0x7A, 0x65, 0x00, 0x31, 0x34, 0x33, 0x32,
///     0x00,
/// ]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferOption {
    /// Type of the option
    pub option: OptionType,
    /// Value of the option
    pub value: u64,
}

impl TransferOption {
    /// Converts a [`TransferOption`] to a [`Vec<u8>`].
    pub fn as_bytes(&self) -> Vec<u8> {
        [
            self.option.as_str().as_bytes(),
            &[0x00],
            self.value.to_string().as_bytes(),
            &[0x00],
        ]
        .concat()
    }
}

/// Wrapper to print TransferOption slices
pub struct OptionFmt<'a>(pub &'a [TransferOption]);
impl fmt::Display for OptionFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i != 0 { write!(f, ", ")? }
            write!(f, "{}:{}", e.option.as_str(), e.value)?;
        }
        Ok(())
    }
}

/// OptionType `enum` represents the TFTP option types
///
/// This `enum` has function implementations for conversion between
/// [`OptionType`]s and [`str`]s.
///
/// # Example
///
/// ```rust
/// use tftpd::OptionType;
///
/// assert_eq!(OptionType::BlockSize, "blksize".parse().unwrap());
/// assert_eq!("tsize", OptionType::TransferSize.as_str());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionType {
    /// Block Size option type
    BlockSize,
    /// Transfer Size option type
    TransferSize,
    /// Timeout option type
    Timeout,
    /// Timeout in ms option type
    TimeoutMs,
    /// Windowsize option type
    WindowSize,
    /// Windowwait option type
    WindowWait,
}

impl OptionType {
    /// Converts an [`OptionType`] to a [`str`].
    pub fn as_str(&self) -> &'static str {
        match self {
            OptionType::BlockSize => "blksize",
            OptionType::TransferSize => "tsize",
            OptionType::Timeout => "timeout",
            OptionType::TimeoutMs => "timeoutms",
            OptionType::WindowSize => "windowsize",
            OptionType::WindowWait => "windowwait",
        }
    }
}

impl FromStr for OptionType {
    type Err = &'static str;

    /// Converts a [`str`] to an [`OptionType`].
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "blksize" => Ok(OptionType::BlockSize),
            "tsize" => Ok(OptionType::TransferSize),
            "timeout" => Ok(OptionType::Timeout),
            "timeoutms" => Ok(OptionType::TimeoutMs),
            "windowsize" => Ok(OptionType::WindowSize),
            "windowwait" => Ok(OptionType::WindowWait),
            _ => Err("Invalid option type"),
        }
    }
}
