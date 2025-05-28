use std::error::Error;
use std::{env, net::SocketAddr, process};
use tftpd::{Client, ClientConfig, Mode, log_err, log_info};

fn main() {
    client(env::args()).unwrap_or_else(|err| {
        log_err!("{err}");
    })
}

fn client<T: Iterator<Item = String>>(args: T) -> Result<(), Box<dyn Error>> {
    // Parse arguments, skipping first one (exec name)
    let config = ClientConfig::new(args.skip(1)).unwrap_or_else(|err| {
        log_err!("Problem parsing arguments: {err}");
        process::exit(1)
    });

    let mut client = Client::new(&config).unwrap_or_else(|err| {
        log_err!("Problem creating client: {err}");
        process::exit(1)
    });

    if config.mode == Mode::Upload {
        log_info!(
            "Starting TFTP Client, uploading {} to {}",
            config.file_path.display(),
            SocketAddr::new(config.remote_ip_address, config.port),
        );
    } else {
        log_info!(
            "Starting TFTP Client, downloading {} from {}",
            config.file_path.display(),
            SocketAddr::new(config.remote_ip_address, config.port),
        );
    }

    client.run()
}
