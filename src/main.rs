use clap::{AppSettings, Clap};
use serial_tcp_bridge;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();

    serial_tcp_bridge::start(&opts.host, opts.port, opts.verbose)
        .unwrap_or_else(|err| eprintln!("Unable to start server: {}", err));
    Ok(())
}

/// Creates a TCP to Serial bridge
#[derive(Clap)]
#[clap(version = "0.1.0", author = "Caleb Fletcher <caleb@fletcher.cf>")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// The IP to listen on
    #[clap(short, long, default_value = "127.0.0.1")]
    host: String,
    /// The port to listen on
    #[clap(short, long, default_value = "41800")]
    port: u16,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}
