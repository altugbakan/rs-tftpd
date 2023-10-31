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

    println!(
        "Running TFTP Server on {}:{} in upload dir {}, download dir {}",
        config.ip_address,
        config.port,
        server.display_updir(),
        server.display_downdir()
    );

    server.listen();
}
