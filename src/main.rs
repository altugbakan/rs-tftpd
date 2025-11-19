use std::{env, net::SocketAddr, process};
use tftpd::{log_err, log_warn, Config, Server};

fn main() {
    server(env::args());
}

fn server<T: Iterator<Item = String>>(args: T) {
    let config = Config::new(args).unwrap_or_else(|err| {
        log_err!("Problem parsing arguments: {err}");
        process::exit(1)
    });

    let mut server = Server::new(&config).unwrap_or_else(|err| {
        log_err!(
            "Problem creating server on {}:{}: {err}",
            config.ip_address,
            config.port
        );
        process::exit(1)
    });

    if config.receive_directory == config.send_directory {
        log_warn!(
            "Running TFTP Server on {} in {}",
            SocketAddr::new(config.ip_address, config.port),
            config.directory.display()
        );
    } else {
        log_warn!(
            "Running TFTP Server on {}. Sending from {}, receiving to {}",
            SocketAddr::new(config.ip_address, config.port),
            config.send_directory.display(),
            config.receive_directory.display(),
        );
    }

    server.listen();
}
