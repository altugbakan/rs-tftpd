use crate::client::Mode;
use crate::server::{
    convert_file_path,
    Rollover,
    DEFAULT_BLOCK_SIZE,
    DEFAULT_WINDOW_SIZE,
    DEFAULT_WINDOW_WAIT,
    DEFAULT_TIMEOUT,
    DEFAULT_MAX_RETRIES,
    DEFAULT_ROLLOVER};
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
    pub window_size: u16,
    /// Inter packets wait delay in windows (default: 10ms)
    pub window_wait: Duration,
    /// Timeout to use during transfer. (default: 5s)
    pub timeout: Duration,
    /// Timeout to use after request. (default: 5s)
    pub timeout_req: Duration,
    /// Max count of retires (default: 6)
    pub max_retries: usize,
    /// Upload or Download a file. (default: Download)
    pub mode: Mode,
    /// Download directory of the TFTP Client. (default: current working directory)
    pub receive_directory: PathBuf,
    /// File to Upload or Download.
    pub file_path: PathBuf,
    /// Should clean (delete) files after receiving errors. (default: true)
    pub clean_on_error: bool,
    /// Block counter roll-over policy  (default: Enforce0)
    pub rollover: Rollover,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            remote_ip_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 69,
            blocksize: DEFAULT_BLOCK_SIZE,
            window_size: DEFAULT_WINDOW_SIZE,
            window_wait: DEFAULT_WINDOW_WAIT,
            timeout: DEFAULT_TIMEOUT,
            timeout_req: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            mode: Mode::Download,
            receive_directory: Default::default(),
            file_path: Default::default(),
            clean_on_error: true,
            rollover: DEFAULT_ROLLOVER,
        }
    }
}


fn parse_duration<T : Iterator<Item = String>>(args : &mut T) -> Result<Duration, Box<dyn Error>> {
    if let Some(dur_str) = args.next() {
        let dur = Duration::from_secs_f32(dur_str.parse::<f32>()?);
        if dur < Duration::from_secs_f32(0.001) {
            Err("duration cannot be shorter than 1 ms".into())
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
                        config.window_size = windowsize_str.parse::<u16>()?;
                    } else {
                        return Err("Missing windowsize after flag".into());
                    }
                }
                "-W" | "--windowwait" => {
                    config.window_wait = parse_duration(&mut args)?;
                }
                "-t" | "--timeout" => {
                    config.timeout = parse_duration(&mut args)?;
                }
                "-T" | "--timeout-req" => {
                    config.timeout_req = parse_duration(&mut args)?;
                }
                "-m" | "--maxretries" => {
                    if let Some(retries_str) = args.next() {
                        config.max_retries = retries_str.parse::<usize>()?;
                    } else {
                        return Err("Missing max retries after flag".into());
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
                "--keep-on-error" => {
                    config.clean_on_error = false;
                }
                "-R" | "--rollover" => {
                    if let Some(arg_str) = args.next() {
                        match arg_str.as_str() {
                            "n" => config.rollover = Rollover::None,
                            "0" => config.rollover = Rollover::Enforce0,
                            "1" => config.rollover = Rollover::Enforce1,
                            "x" => config.rollover = Rollover::DontCare,
                            _ => return Err("Invalid rollover policy value: use n, 0, 1, x".into()),
                        }
                    } else {
                        return Err("Rollover policy value missing: use n, 0, 1, x".into())
                    }
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
                    println!("  -m, --maxretries <cnt>\t\tset the max retries count (default: 6)");
                    println!("  -R, --rollover <policy>\t\tsets the rollover policy: 0, 1, n (forbidden), x (don't care) (default: 0)");
                    println!("  -u, --upload\t\t\t\tselect upload mode, ignores previous flags");
                    println!("  -d, --download\t\t\tselect download mode, ignores previous flags");
                    println!("  -rd, --receive-directory <DIR>\tdirectory to receive files when in Download mode (default: current)");
                    println!("  --keep-on-error\t\t\tprevent client from deleting files after receiving errors");
                    println!("  -h, --help\t\t\t\tprint help information");
                    process::exit(0);
                }
                "--" => {
                    while let Some(arg) = args.next() {
                        if !config.file_path.as_os_str().is_empty() {
                            return Err("too many arguments".into());
                        }
                        config.file_path = convert_file_path(arg.as_str());
                    }
                }
                filename => {
                    if !config.file_path.as_os_str().is_empty() {
                        return Err("too many arguments".into());
                    }

                    if filename.starts_with('-') {
                        return Err(format!("unkwon flag {filename} (or use '--' to force into filename)").into());
                    }
                    config.file_path = convert_file_path(filename);
                }
            }
        }

        if config.file_path.as_os_str().is_empty() {
            return Err("missing filename".into());
        }

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
        assert_eq!(config.window_size, 2);
        assert_eq!(config.window_wait, Duration::from_millis(20));
        assert_eq!(config.blocksize, 1024);
        assert_eq!(config.mode, Mode::Upload);
        assert_eq!(config.timeout, Duration::from_secs(4));
        assert!(!config.clean_on_error);
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
        assert_eq!(config.blocksize, 2048);
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
