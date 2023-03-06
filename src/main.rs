use std::{env, process};
use tftpd::Config;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1)
    });

    println!(
        "Running TFTP Server on {}:{}",
        config.ip_address, config.port
    );
}
