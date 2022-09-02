use crate::component::Component;
use crate::Error;

pub(crate) trait Collector {
    fn setup(&mut self) -> Result<(), Error>;
    fn collect(&self) -> Result<Vec<Component>, Error>;
}

pub(crate) fn get() -> Result<Box<dyn Collector>, Error> {
    if cfg!(target_os = "macos") {
        use crate::macos;

        let mut coll = macos::Collector::default();

        coll.setup()?;

        return Ok(Box::new(coll));
    }

    Err("unsupported operating system".to_string())
}
