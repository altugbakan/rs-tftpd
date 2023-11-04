use std::{env, process};
use tftpd::{Config, Server};

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
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
            "Running TFTP Server on {}:{} in {}",
            config.ip_address,
            config.port,
            config.directory.display()
        );
    } else {
        println!(
            "Running TFTP Server on {}:{}. Sending from {}, receiving to {}",
            config.ip_address,
            config.port,
            config.send_directory.display(),
            config.receive_directory.display(),
        );
    }

    server.listen();
}
