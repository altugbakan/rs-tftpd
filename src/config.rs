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
}

impl Config {
    /// Creates a new configuration by parsing the supplied arguments. It is
    /// intended for use with [`env::args()`].
    pub fn new<T>(mut args: T) -> Result<Config, Box<dyn Error>>
    where
        T: Iterator<Item = String>,
    {
        let mut config = Config {
            ip_address: Ipv4Addr::new(127, 0, 0, 1),
            port: 69,
            directory: env::current_dir().unwrap_or_else(|_| env::temp_dir()),
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
                "-h" | "--help" => {
                    println!("TFTP Server Daemon\n");
                    println!("Usage: tftpd [OPTIONS]\n");
                    println!("Options:");
                    println!("  -i, --ip-address <IP ADDRESS>\tSet the ip address of the server (default: 127.0.0.1)");
                    println!(
                        "  -p, --port <PORT>\t\tSet the listening port of the server (default: 69)"
                    );
                    println!("  -d, --directory <DIRECTORY>\tSet the listening port of the server (default: Current Working Directory)");
                    println!("  -h, --help\t\t\tPrint help information");
                    process::exit(0);
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
            vec!["/", "-i", "0.0.0.0", "-p", "1234", "-d", "/"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();

        assert_eq!(config.ip_address, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.port, 1234);
        assert_eq!(config.directory, PathBuf::from_str("/").unwrap());
    }

    #[test]
    fn parses_some_config() {
        let config = Config::new(
            vec!["/", "-i", "0.0.0.0", "-d", "/"]
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
            vec!["/", "-i", "1234.5678.9012.3456"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }

    #[test]
    fn returns_error_on_invalid_port() {
        assert!(Config::new(vec!["/", "-p", "1234567"].iter().map(|s| s.to_string()),).is_err());
    }

    #[test]
    fn returns_error_on_invalid_directory() {
        assert!(Config::new(
            vec!["/", "-d", "/this/does/not/exist"]
                .iter()
                .map(|s| s.to_string()),
        )
        .is_err());
    }
}
