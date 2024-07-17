use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::{env, process};

/// Configuration `struct` used for parsing TFTP options from user
/// input.
///
/// This `struct` is meant to be created by [`Config::new()`]. See its
/// documentation for more.
///
/// # Example
///
/// ```rust
/// // Create TFTP configuration from user arguments.
/// use std::env;
/// use tftpd::Config;
///
/// let config = Config::new(env::args()).unwrap();
/// ```
pub struct Config {
    /// Local IP address of the TFTP Server. (default: 127.0.0.1)
    pub ip_address: IpAddr,
    /// Local Port number of the TFTP Server. (default: 69)
    pub port: u16,
    /// Default directory of the TFTP Server. (default: current working directory)
    pub directory: PathBuf,
    /// Upload directory of the TFTP Server. (default: directory)
    pub receive_directory: PathBuf,
    /// Download directory of the TFTP Server. (default: directory)
    pub send_directory: PathBuf,
    /// Use a single port for both sending and receiving. (default: false)
    pub single_port: bool,
    /// Refuse all write requests, making the server read-only. (default: false)
    pub read_only: bool,
    /// Duplicate all packets sent from the server. (default: 0)
    pub duplicate_packets: u8,
    /// Overwrite existing files. (default: false)
    pub overwrite: bool,
    /// Should clean (delete) files after receiving errors. (default: true)
    pub clean_on_error: bool
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 69,
            directory: env::current_dir().unwrap_or_else(|_| env::temp_dir()),
            receive_directory: Default::default(),
            send_directory: Default::default(),
            single_port: Default::default(),
            read_only: Default::default(),
            duplicate_packets: Default::default(),
            overwrite: Default::default(),
            clean_on_error: true
        }
    }
}

impl Config {
    /// Creates a new configuration by parsing the supplied arguments. It is
    /// intended for use with [`env::args()`].
    pub fn new<T: Iterator<Item = String>>(mut args: T) -> Result<Config, Box<dyn Error>> {
        let mut config = Config::default();

        args.next();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ip-address" => {
                    if let Some(ip_str) = args.next() {
                        let ip_addr: IpAddr = ip_str.parse()?;
                        config.ip_address = ip_addr;
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
                "-d" | "--directory" => {
                    if let Some(dir_str) = args.next() {
                        if !Path::new(&dir_str).exists() {
                            return Err(format!("{dir_str} does not exist").into());
                        }
                        config.directory = dir_str.into();
                    } else {
                        return Err("Missing directory after flag".into());
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
                "-sd" | "--send-directory" => {
                    if let Some(dir_str) = args.next() {
                        if !Path::new(&dir_str).exists() {
                            return Err(format!("{dir_str} does not exist").into());
                        }
                        config.send_directory = dir_str.into();
                    } else {
                        return Err("Missing send directory after flag".into());
                    }
                }
                "-s" | "--single-port" => {
                    config.single_port = true;
                }
                "-r" | "--read-only" => {
                    config.read_only = true;
                }
                "-h" | "--help" => {
                    println!("TFTP Server Daemon\n");
                    println!("Usage: tftpd [OPTIONS]\n");
                    println!("Options:");
                    println!("  -i, --ip-address <IP ADDRESS>\t\tSet the ip address of the server (default: 127.0.0.1)");
                    println!(
                        "  -p, --port <PORT>\t\t\tSet the listening port of the server (default: 69)"
                    );
                    println!("  -d, --directory <DIRECTORY>\t\tSet the serving directory (default: current working directory)");
                    println!("  -rd, --receive-directory <DIRECTORY>\tSet the directory to receive files to (default: the directory setting)");
                    println!("  -sd, --send-directory <DIRECTORY>\tSet the directory to send files from (default: the directory setting)");
                    println!("  -s, --single-port\t\t\tUse a single port for both sending and receiving (default: false)");
                    println!("  -r, --read-only\t\t\tRefuse all write requests, making the server read-only (default: false)");
                    println!("  --duplicate-packets <NUM>\t\tDuplicate all packets sent from the server (default: 0)");
                    println!("  --overwrite\t\t\t\tOverwrite existing files (default: false)");
                    println!("  --keep-on-error\t\t\t\tPrevent daemon from deleting files after receiving errors");
                    println!("  -h, --help\t\t\t\tPrint help information");
                    process::exit(0);
                }
                "--duplicate-packets" => {
                    if let Some(duplicate_packets_str) = args.next() {
                        let duplicate_packets = duplicate_packets_str.parse::<u8>()?;

                        if duplicate_packets == u8::MAX {
                            return Err(format!(
                                "Duplicate packets should be less than {}",
                                u8::MAX
                            )
                            .into());
                        }
                        config.duplicate_packets = duplicate_packets;
                    } else {
                        return Err("Missing duplicate packets after flag".into());
                    }
                }
                "--overwrite" => {
                    config.overwrite = true;
                }
                "--keep-on-error" => {
                    config.clean_on_error = false;
                }

                invalid => return Err(format!("Invalid flag: {invalid}").into()),
            }
        }

        if config.receive_directory.as_os_str().is_empty() {
            config.receive_directory.clone_from(&config.directory);
        }
        if config.send_directory.as_os_str().is_empty() {
            config.send_directory.clone_from(&config.directory);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn parses_full_config() {
        let config = Config::new(
            [
                "/", "-i", "0.0.0.0", "-p", "1234", "-d", "/", "-rd", "/", "-sd", "/", "-s", "-r", "--dont-clean"
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 1234);
        assert_eq!(config.directory, PathBuf::from("/"));
        assert_eq!(config.receive_directory, PathBuf::from("/"));
        assert_eq!(config.send_directory, PathBuf::from("/"));
        assert_eq!(config.clean_on_error, false);
        assert!(config.single_port);
        assert!(config.read_only);
    }

    #[test]
    fn parses_config_with_ipv6() {
        let config = Config::new(
            ["/", "-i", "0:0:0:0:0:0:0:0", "-p", "1234"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.ip_address, Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0));
        assert_eq!(config.port, 1234);
    }

    #[test]
    fn parses_some_config() {
        let config = Config::new(
            ["/", "-i", "0.0.0.0", "-d", "/"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 69);
        assert_eq!(config.directory, PathBuf::from("/"));
    }

    #[test]
    fn sets_receive_directory_to_directory() {
        let config = Config::new(["/", "-d", "/"].iter().map(|s| s.to_string())).unwrap();

        assert_eq!(config.receive_directory, PathBuf::from("/"));
    }

    #[test]
    fn sets_send_directory_to_directory() {
        let config = Config::new(["/", "-d", "/"].iter().map(|s| s.to_string())).unwrap();

        assert_eq!(config.send_directory, PathBuf::from("/"));
    }

    #[test]
    fn returns_error_on_invalid_ip() {
        assert!(Config::new(
            ["/", "-i", "1234.5678.9012.3456"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_invalid_port() {
        assert!(Config::new(["/", "-p", "1234567"].iter().map(|s| s.to_string()),).is_err());
    }

    #[test]
    fn returns_error_on_invalid_directory() {
        assert!(Config::new(
            ["/", "-d", "/this/does/not/exist"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_invalid_up_directory() {
        assert!(Config::new(
            ["/", "-ud", "/this/does/not/exist"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_invalid_down_directory() {
        assert!(Config::new(
            ["/", "-dd", "/this/does/not/exist"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_invalid_duplicate_packets() {
        assert!(Config::new(
            ["/", "--duplicate-packets", "-1"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_max_duplicate_packets() {
        assert!(Config::new(
            ["/", "--duplicate-packets", format!("{}", u8::MAX).as_str()]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn initializes_duplicate_packets_as_zero() {
        let config = Config::new(["/"].iter().map(|s| s.to_string())).unwrap();

        assert_eq!(config.duplicate_packets, 0);
    }
}
