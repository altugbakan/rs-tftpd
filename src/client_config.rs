use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;

use crate::client::Mode;
use crate::server::convert_file_path;
use crate::config;
use crate::options::{DEFAULT_TIMEOUT, OptionsProtocol, OptionsPrivate};
use crate::log::*;

#[cfg(feature = "debug_drop")]
use crate::drop::drop_set;

/// Configuration `struct` used for parsing TFTP Client options from user
/// input.
///
/// This `struct` is meant to be created by [`ClientConfig::new()`]. See its
/// documentation for more.
///
/// # Example
///
/// ```rust
/// // Create TFTP configuration from user arguments.
/// use std::env;
/// use tftpd::ClientConfig;
///
/// let client_config = ClientConfig::new(env::args());
/// ```
#[derive(Debug)]
pub struct ClientConfig {
    /// Local IP address of the TFTP Client. (default: 127.0.0.1)
    pub remote_ip_address: IpAddr,
    /// Local Port number of the TFTP Client. (default: 69)
    pub port: u16,
    /// Timeout to use after request. (default: 5s)
    pub timeout_req: Duration,
    /// Upload or Download a file. (default: Download)
    pub mode: Mode,
    /// Download directory of the TFTP Client. (default: current working directory)
    pub receive_directory: PathBuf,
    /// File to Upload or Download.
    pub file_path: PathBuf,
    /// Local options for client
    pub opt_local: OptionsPrivate,
    /// Common options for client
    pub opt_common: OptionsProtocol,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            remote_ip_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 69,
            timeout_req: DEFAULT_TIMEOUT,
            mode: Mode::Download,
            receive_directory: Default::default(),
            file_path: Default::default(),
            opt_local: Default::default(),
            opt_common: Default::default(),
        }
    }
}

fn parse_duration<T : Iterator<Item = String>>(args : &mut T) -> Result<Duration, Box<dyn Error>> {
    if let Some(dur_str) = args.next() {
        let dur = Duration::from_secs_f32(dur_str.parse::<f32>()?);
        if dur < Duration::from_secs_f32(0.001) {
            Err("duration cannot be shorter than 1 ms".into())
        } else if dur > Duration::from_secs(255) {
            Err("duration cannot be greater than 255 s".into())
        } else {
            Ok(dur)
        }
    } else {
        Err("Missing duration after flag".into())
    }
}

impl ClientConfig {
    /// Creates a new configuration by parsing the supplied arguments. It is
    /// intended for use with [`env::args()`].
    pub fn new<T: Iterator<Item = String>>(mut args: T) -> Result<ClientConfig, Box<dyn Error>> {
        let mut config = ClientConfig::default();
        let mut verbosity : isize = 1;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ip-address" => {
                    if let Some(ip_str) = args.next() {
                        let ip_addr: IpAddr = ip_str.parse()?;
                        config.remote_ip_address = ip_addr;
                    } else {
                        return Err("Missing ip address after flag".into());
                    }
                }
                "-p" | "--port" => {
                    if let Some(port_str) = args.next() {
                        config.port = port_str.parse::<u16>()?;
                    } else {
                        return Err("Missing port number after flag".into());
                    }
                }
                "-b" | "--blocksize" => {
                    if let Some(blocksize_str) = args.next() {
                        config.opt_common.block_size = blocksize_str.parse::<u16>()?;
                    } else {
                        return Err("Missing blocksize after flag".into());
                    }
                }
                "-w" | "--windowsize" => {
                    if let Some(windowsize_str) = args.next() {
                        config.opt_common.window_size = windowsize_str.parse::<u16>()?;
                    } else {
                        return Err("Missing windowsize after flag".into());
                    }
                }
                "-W" | "--windowwait" => {
                    config.opt_common.window_wait = parse_duration(&mut args)?;
                }
                "-t" | "--timeout" => {
                    config.opt_common.timeout = parse_duration(&mut args)?;
                }
                "-T" | "--timeout-req" => {
                    config.timeout_req = parse_duration(&mut args)?;
                }
                "-rd" | "--receive-directory" => {
                    if let Some(dir_str) = args.next() {
                        if !Path::new(&dir_str).exists() {
                            return Err(format!("{dir_str} does not exist").into());
                        }
                        config.receive_directory = dir_str.into();
                    } else {
                        return Err("Missing receive directory after flag".into());
                    }
                }
                "-u" | "--upload" => {
                    config.mode = Mode::Upload;
                }
                "-d" | "--download" => {
                    config.mode = Mode::Download;
                }
                "-h" | "--help" => {
                    println!("TFTP Client\n");
                    println!("Usage: tftpd client <File> [OPTIONS]\n");
                    println!("Options:");
                    println!("  -i, --ip-address <IP ADDRESS>\t\tIP address of the server (default: 127.0.0.1)");
                    println!("  -p, --port <PORT>\t\t\tUDP port of the server (default: 69)");
                    println!("  -b, --blocksize <number>\t\tset the blocksize (default: 512)");
                    println!("  -w, --windowsize <number>\t\tset the windowsize (default: 1)");
                    println!("  -W, --windowwait <seconds>\t\t inter-packet wait time in seconds for windows (default: 0.01)");
                    println!("  -t, --timeout <seconds>\t\tset the timeout for data in seconds (default: 5, can be float)");
                    println!("  -T, --timeout-req <seconds>\t\tset the timeout after request in seconds (default: 5, can be float)");
                    println!("  -u, --upload\t\t\t\tselect upload mode, ignores previous flags");
                    println!("  -d, --download\t\t\tselect download mode, ignores previous flags");
                    println!("  -rd, --receive-directory <DIR>\tdirectory to receive files when in Download mode (default: current)");
                    config::print_opt_local_help();
                    println!("  -h, --help\t\t\t\tprint help information");
                    process::exit(0);
                }
                "-q" | "--quiet" => verbosity -= 1,
                "-v" | "--verbose" => verbosity += 1,
                #[cfg(feature = "debug_drop")]
                "-D" => drop_set(args.next())?,
                "--" => {
                    while let Some(arg) = args.next() {
                        if !config.file_path.as_os_str().is_empty() {
                            return Err("too many arguments".into());
                        }
                        config.file_path = convert_file_path(arg.as_str());
                    }
                }
                arg => if !config::parse_local_args(arg, &mut args, &mut config.opt_local)? {
                    if !config.file_path.as_os_str().is_empty() {
                        return Err("too many arguments".into());
                    }

                    if arg.starts_with('-') {
                        return Err(format!("unkwon flag {arg} (or use '--' to force into filename)").into());
                    }
                    config.file_path = convert_file_path(arg);
                }
            }
        }

        if config.file_path.as_os_str().is_empty() {
            return Err("missing filename".into());
        }

        verbosity_set(verbosity);

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let config = ClientConfig::new(
            [
                "test.file",
                "-i",
                "0.0.0.0",
                "-p",
                "1234",
                "-rd",
                "/",
                "-d",
                "-u",
                "-b",
                "1024",
                "-w",
                "2",
                "-W",
                "0.02",
                "-t",
                "4",
                "--keep-on-error",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.remote_ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 1234);
        assert_eq!(config.receive_directory, PathBuf::from("/"));
        assert_eq!(config.file_path, PathBuf::from("test.file"));
        assert_eq!(config.opt_common.window_size, 2);
        assert_eq!(config.opt_common.window_wait, Duration::from_millis(20));
        assert_eq!(config.opt_common.block_size, 1024);
        assert_eq!(config.mode, Mode::Upload);
        assert_eq!(config.opt_common.timeout, Duration::from_secs(4));
        assert!(!config.opt_local.clean_on_error);
    }

    #[test]
    fn parses_partial_config() {
        let config = ClientConfig::new(
            ["test.file", "-d", "-b", "2048", "-p", "2000"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.port, 2000);
        assert_eq!(config.file_path, PathBuf::from("test.file"));
        assert_eq!(config.opt_common.block_size, 2048);
        assert_eq!(config.mode, Mode::Download);
    }

    #[test]
    fn parses_file_paths() {
        let config =
            ClientConfig::new(["test/test.file"].iter().map(|s| s.to_string())).unwrap();

        let mut path = PathBuf::new();
        path.push("test");
        path.push("test.file");

        assert_eq!(config.file_path, path);

        let config = ClientConfig::new(
            ["test\\test\\test.file"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        let mut path = PathBuf::new();
        path.push("test");
        path.push("test");
        path.push("test.file");

        assert_eq!(config.file_path, path);
    }
}
