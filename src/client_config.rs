use crate::client::Mode;
use crate::packet::{DEFAULT_BLOCKSIZE, DEFAULT_TIMEOUT, DEFAULT_WINDOWSIZE};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;

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
    /// Blocksize to use during transfer. (default: 512)
    pub blocksize: usize,
    /// Windowsize to use during transfer. (default: 1)
    pub windowsize: u16,
    /// Timeout to use during transfer. (default: 5s)
    pub timeout: Duration,
    /// Upload or Download a file. (default: Download)
    pub mode: Mode,
    /// Download directory of the TFTP Client. (default: current working directory)
    pub receive_directory: PathBuf,
    /// File to Upload or Download.
    pub filename: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            remote_ip_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 69,
            blocksize: DEFAULT_BLOCKSIZE,
            windowsize: DEFAULT_WINDOWSIZE,
            timeout: DEFAULT_TIMEOUT,
            mode: Mode::Download,
            receive_directory: Default::default(),
            filename: Default::default(),
        }
    }
}

impl ClientConfig {
    /// Creates a new configuration by parsing the supplied arguments. It is
    /// intended for use with [`env::args()`].
    pub fn new<T: Iterator<Item = String>>(mut args: T) -> Result<ClientConfig, Box<dyn Error>> {
        let mut config = ClientConfig::default();

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
                        config.blocksize = blocksize_str.parse::<usize>()?;
                    } else {
                        return Err("Missing blocksize after flag".into());
                    }
                }
                "-w" | "--windowsize" => {
                    if let Some(windowsize_str) = args.next() {
                        config.windowsize = windowsize_str.parse::<u16>()?;
                    } else {
                        return Err("Missing windowsize after flag".into());
                    }
                }
                "-t" | "--timeout" => {
                    if let Some(timeout_str) = args.next() {
                        config.timeout = Duration::from_secs(timeout_str.parse::<u64>()?);
                    } else {
                        return Err("Missing timeout after flag".into());
                    }
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
                    println!("  -i, --ip-address <IP ADDRESS>\t\tIp address of the server (default: 127.0.0.1)");
                    println!("  -p, --port <PORT>\t\t\tPort of the server (default: 69)");
                    println!("  -b, --blocksize <number>\t\tSets the blocksize (default: 512)");
                    println!("  -w, --windowsize <number>\t\tSets the windowsize (default: 1)");
                    println!(
                        "  -t, --timeout <seconds>\t\tSets the timeout in seconds (default: 5)"
                    );
                    println!("  -u, --upload\t\t\t\tSets the client to upload mode, Ignores all previous download flags");
                    println!("  -d, --download\t\t\tSet the client to download mode, Invalidates all previous upload flags");
                    println!("  -rd, --receive-directory <DIRECTORY>\tSet the directory to receive files when in Download mode (default: current working directory)");
                    println!("  -h, --help\t\t\t\tPrint help information");
                    process::exit(0);
                }
                file_name => {
                    config.filename = PathBuf::from(file_name);
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use ClientConfig;

    use super::*;

    #[test]
    fn parses_full_config() {
        let config = ClientConfig::new(
            [
                "client",
                "test.file",
                "-i",
                "0.0.0.0",
                "-p",
                "1234",
                "-sd",
                "/",
                "-d",
                "-u",
                "-b",
                "1024",
                "-w",
                "2",
                "-t",
                "4",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.remote_ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 1234);
        assert_eq!(config.receive_directory, PathBuf::from("/"));
        assert_eq!(config.filename, PathBuf::from("test.file"));
        assert_eq!(config.windowsize, 2);
        assert_eq!(config.blocksize, 1024);
        assert_eq!(config.mode, Mode::Upload);
        assert_eq!(config.timeout, Duration::from_secs(4));
    }

    #[test]
    fn parses_partial_config() {
        let config = ClientConfig::new(
            ["client", "test.file", "-d", "-b", "2048", "-p", "2000"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.port, 2000);
        assert_eq!(config.filename, PathBuf::from("test.file"));
        assert_eq!(config.blocksize, 2048);
        assert_eq!(config.mode, Mode::Download);
    }
}
