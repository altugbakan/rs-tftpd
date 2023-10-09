use std::error::Error;
use std::net::Ipv4Addr;
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
    pub ip_address: Ipv4Addr,
    /// Local Port number of the TFTP Server. (default: 69)
    pub port: u16,
    /// Default directory of the TFTP Server. (default: current working directory)
    pub directory: PathBuf,
    /// Use a single port for both sending and receiving. (default: false)
    pub single_port: bool,
    /// Refuse all write requests, making the server read-only. (default: false)
    pub read_only: bool,
    /// Duplicate all packets sent from the server. (default: 1)
    pub duplicate_packets: u8,
    /// Overwrite existing files. (default: false)
    pub overwrite: bool,
}

impl Config {
    /// Creates a new configuration by parsing the supplied arguments. It is
    /// intended for use with [`env::args()`].
    pub fn new<T: Iterator<Item = String>>(mut args: T) -> Result<Config, Box<dyn Error>> {
        let mut config = Config {
            ip_address: Ipv4Addr::new(127, 0, 0, 1),
            port: 69,
            directory: env::current_dir().unwrap_or_else(|_| env::temp_dir()),
            single_port: false,
            read_only: false,
            duplicate_packets: 1,
            overwrite: false,
        };

        args.next();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ip-address" => {
                    if let Some(ip_str) = args.next() {
                        config.ip_address = ip_str.parse::<Ipv4Addr>()?;
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
                        config.directory = PathBuf::from(dir_str);
                    } else {
                        return Err("Missing directory after flag".into());
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
                    println!("  -i, --ip-address <IP ADDRESS>\tSet the ip address of the server (default: 127.0.0.1)");
                    println!(
                        "  -p, --port <PORT>\t\tSet the listening port of the server (default: 69)"
                    );
                    println!("  -d, --directory <DIRECTORY>\tSet the serving directory (default: Current Working Directory)");
                    println!("  -s, --single-port\t\tUse a single port for both sending and receiving (default: false)");
                    println!("  -r, --read-only\t\tRefuse all write requests, making the server read-only (default: false)");
                    println!("  --duplicate-packets <NUM>\tDuplicate all packets sent from the server (default: 0)");
                    println!("  --overwrite\t\t\tOverwrite existing files (default: false)");
                    println!("  -h, --help\t\t\tPrint help information");
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

                invalid => return Err(format!("Invalid flag: {invalid}").into()),
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parses_full_config() {
        let config = Config::new(
            ["/", "-i", "0.0.0.0", "-p", "1234", "-d", "/", "-s", "-r"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 1234);
        assert_eq!(config.directory, PathBuf::from_str("/").unwrap());
        assert!(config.single_port);
        assert!(config.read_only);
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
        assert_eq!(config.directory, PathBuf::from_str("/").unwrap());
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
}
