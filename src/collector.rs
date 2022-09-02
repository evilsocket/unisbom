use crate::component::Component;
use crate::Error;

pub(crate) trait Collector {
    fn setup(&mut self) -> Result<(), Error>;
    fn collect(&self) -> Result<Vec<Box<dyn Component>>, Error>;
    fn collect_from_json(&self, json: &str) -> Result<Vec<Box<dyn Component>>, Error>;
}

#[cfg(target_os = "macos")]
pub(crate) fn get() -> Result<Box<dyn Collector>, Error> {
    use crate::macos;

    let mut coll = macos::Collector::default();

    coll.setup()?;

    Ok(Box::new(coll))
}

#[cfg(target_os = "windows")]
pub(crate) fn get() -> Result<Box<dyn Collector>, Error> {
    use crate::windows;

    let mut coll = windows::Collector::default();

    coll.setup()?;

    Ok(Box::new(coll))
}

#[cfg(target_os = "linux")]
pub(crate) fn get() -> Result<Box<dyn Collector>, Error> {
    Err("unsupported operating system".to_string())
}
