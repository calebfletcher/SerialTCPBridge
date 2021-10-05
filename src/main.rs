use clap::{crate_authors, crate_version, AppSettings, Clap};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();
    env_logger::init();

    serial_tcp_bridge::start(&opts.host, opts.port, &opts.device)
        .unwrap_or_else(|err| eprintln!("Unable to start server: {}", err));
    Ok(())
}

/// Creates a TCP to Serial bridge
#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// The IP to listen on
    #[clap(short, long, default_value = "127.0.0.1")]
    host: String,
    /// The port to listen on
    #[clap(short, long, default_value = "41800")]
    port: u16,
    /// The serial port to connect to
    #[clap(short, long)]
    device: String,
}
