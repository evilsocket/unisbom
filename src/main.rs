use clap::Parser;

pub(crate) type Error = String;

mod collector;
mod component;
mod format;
mod utils;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(clap::ValueEnum, Default, Debug, Clone)]
enum OutputFormat {
    #[default]
    Text,
    JSON,
}

#[derive(Parser, Default, Debug, Clone)]
#[clap(about = "Build a software bill of materials of the current system.")]
struct Arguments {
    /// Specify output format, text will print a summary of each component, while JSON will dump the full information.
    #[clap(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

fn main() -> Result<(), Error> {
    let args = Arguments::parse();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let components = collector::get()?.collect()?;

    match args.format {
        OutputFormat::Text => format::to_text(components, std::io::stdout())?,
        OutputFormat::JSON => format::to_json(components, std::io::stdout())?,
    }

    Ok(())
}
