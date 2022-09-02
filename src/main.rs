use std::env;

pub(crate) type Error = String;

mod collector;
mod component;
mod utils;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

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
    // let json = std::fs::read_to_string("./test-macos.json").unwrap();
    // let components = collector::get()?.collect_from_json(&json)?;

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

    Ok(())
}
