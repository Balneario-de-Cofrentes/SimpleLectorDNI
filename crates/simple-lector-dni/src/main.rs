use clap::Parser;
use simple_lector_dni::app;
use simple_lector_dni::cli::Cli;

fn main() {
    if let Err(error) = app::run(Cli::parse()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
