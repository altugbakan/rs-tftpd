use std::{env, error::Error, net::SocketAddr, process, process::ExitCode};
use tftpd::{log_err, log_info, Client, ClientConfig, Mode};

fn main() -> ExitCode {
    match client(env::args()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            log_err!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn client<T: Iterator<Item = String>>(args: T) -> Result<bool, Box<dyn Error>> {
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
