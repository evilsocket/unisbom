use std::env;

pub(crate) type Error = String;

mod collector;
mod component;
mod utils;

#[cfg(target_os = "macos")]
mod macos;

fn main() -> Result<(), Error> {
    print!(
        "{} v{}\n\n",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let components = collector::get()?.collect()?;

    for comp in components {
        println!(
            "[{:?}] {} {} ({})",
            comp.kind(),
            comp.name(),
            comp.version(),
            comp.path()
        );
    }

    Ok(())
}
