#[cfg(feature = "client")]
use std::error::Error;
use std::{env, net::SocketAddr, process};
#[cfg(not(feature = "client"))]
use tftpd::{Config, Server};
#[cfg(feature = "client")]
use tftpd::{Client, ClientConfig, Config, Mode, Server};

#[cfg(feature = "client")]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{}: incorrect usage", args[0]);
        eprintln!("{} <client | server> [args]", args[0]);
    } else if args[1] == "client" {
        client(args[1..].iter().map(|s| s.to_string())).unwrap_or_else(|err| {
            eprintln!("{err}");
        })
    } else if args[1] == "server" {
        server(args[1..].iter().map(|s| s.to_string()));
    } else {
        eprintln!("{}: incorrect usage", args[0]);
        eprintln!("{} (client | server) [args]", args[0]);
    }
}


#[cfg(not(feature = "client"))]
fn main() {
    let args: Vec<String> = env::args().collect();
    server(args[0..].iter().map(|s| s.to_string()));
}

#[cfg(feature = "client")]
fn client<T: Iterator<Item = String>>(args: T) -> Result<(), Box<dyn Error>> {
    let config = ClientConfig::new(args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1)
    });

    let mut server = Client::new(&config).unwrap_or_else(|err| {
        eprintln!("Problem creating client: {err}");
        process::exit(1)
    });

    if config.mode == Mode::Upload {
        println!(
            "Starting TFTP Client, uploading {} to {}",
            config.filename.display(),
            SocketAddr::new(config.remote_ip_address, config.port),
        );
    } else {
        println!(
            "Starting TFTP Client, downloading {} to {}",
            config.filename.display(),
            SocketAddr::new(config.remote_ip_address, config.port),
        );
    }

    server.start()
}

fn server<T: Iterator<Item = String>>(args: T) {
    let config = Config::new(args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1)
    });

    let mut server = Server::new(&config).unwrap_or_else(|err| {
        eprintln!(
            "Problem creating server on {}:{}: {err}",
            config.ip_address, config.port
        );
        process::exit(1)
    });

    if config.receive_directory == config.send_directory {
        println!(
            "Running TFTP Server on {} in {}",
            SocketAddr::new(config.ip_address, config.port),
            config.directory.display()
        );
    } else {
        println!(
            "Running TFTP Server on {}. Sending from {}, receiving to {}",
            SocketAddr::new(config.ip_address, config.port),
            config.send_directory.display(),
            config.receive_directory.display(),
        );
    }

    server.listen();
}
