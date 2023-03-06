use std::error::Error;
use std::net::{AddrParseError, Ipv4Addr};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::{env, fmt, process};

pub struct Config {
    pub ip_address: Ipv4Addr,
    pub port: u16,
    pub directory: PathBuf,
}

impl Config {
    pub fn new<T>(mut args: T) -> Result<Config, ConfigError>
    where
        T: Iterator<Item = String>,
    {
        let mut config = Config {
            ip_address: Ipv4Addr::new(127, 0, 0, 1),
            port: 69,
            directory: env::current_dir().unwrap_or(env::temp_dir()),
        };

        args.next();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ip-address" => {
                    if let Some(ip_str) = args.next() {
                        config.ip_address = ip_str.parse::<Ipv4Addr>()?;
                    } else {
                        return Err("missing ip address after flag".into());
                    }
                }
                "-p" | "--port" => {
                    if let Some(port_str) = args.next() {
                        config.port = port_str.parse::<u16>()?;
                    } else {
                        return Err("missing port number after flag".into());
                    }
                }
                "-d" | "--directory" => {
                    if let Some(dir_str) = args.next() {
                        if !Path::new(&dir_str).exists() {
                            return Err(format!("{} does not exist", dir_str).into());
                        }
                        config.directory = PathBuf::from(dir_str);
                    } else {
                        return Err("missing directory after flag".into());
                    }
                }
                "-h" | "--help" => {
                    println!("TFTP Server Daemon\n");
                    println!("Usage: tftpd [OPTIONS]\n");
                    println!("Options:");
                    println!("  -i, --ip-address <IP ADDRESS>\tSet the ip address of the server (Default: 127.0.0.1)");
                    println!(
                        "  -p, --port <PORT>\t\tSet the listening port of the server (Default: )"
                    );
                    println!("  -d, --directory <DIRECTORY>\tSet the listening port of the server (Default: )");
                    println!("  -h, --help\t\t\tPrint help information");
                    process::exit(0);
                }
                invalid => return Err(format!("invalid flag: {}", invalid).into()),
            }
        }

        Ok(config)
    }
}

#[derive(Debug)]
pub struct ConfigError {
    description: String,
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl From<AddrParseError> for ConfigError {
    fn from(value: AddrParseError) -> Self {
        ConfigError {
            description: value.to_string(),
        }
    }
}

impl From<ParseIntError> for ConfigError {
    fn from(value: ParseIntError) -> Self {
        ConfigError {
            description: value.to_string(),
        }
    }
}

impl From<String> for ConfigError {
    fn from(value: String) -> Self {
        ConfigError { description: value }
    }
}

impl From<&str> for ConfigError {
    fn from(value: &str) -> Self {
        ConfigError {
            description: value.to_string(),
        }
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
