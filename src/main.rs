use std::env;

use clap::Parser;

pub(crate) type Error = String;

mod collector;
mod component;
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

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let components = collector::get()?.collect()?;

    match args.format {
        OutputFormat::Text => {
            for comp in components {
                println!(
                    "<{}> [{:?}] id={} name={} version={} path={}",
                    comp.modified(),
                    comp.kind(),
                    comp.id(),
                    comp.name(),
                    comp.version(),
                    comp.path()
                );
            }
        }
        OutputFormat::JSON => {
            let json = serde_json::to_string(&components)
                .map_err(|e| format!("can't serialize to json: {:?}", e))?;

            println!("{}", json);
        }
    }

    Ok(())
}
