use crate::component::{Component, ComponentTrait};
use crate::Error;

pub(crate) fn to_text<T: std::io::Write>(
    components: Vec<Box<dyn ComponentTrait>>,
    mut writer: T,
) -> Result<(), Error> {
    for comp in components {
        writer
            .write_all(
                format!(
                    "<{}> [{:?}] name={} version={} path={}\n",
                    comp.modified(),
                    comp.kind(),
                    comp.name(),
                    comp.version(),
                    comp.path()
                )
                .as_bytes(),
            )
            .map_err(|e| format!("can't write text to output: {:?}", e))?;
    }

    Ok(())
}

pub(crate) fn to_json<T: std::io::Write>(
    components: Vec<Box<dyn ComponentTrait>>,
    mut writer: T,
) -> Result<(), Error> {
    let serializable: Vec<Component> = components
        .iter()
        .map(|c| Component::from_trait(c.as_ref()))
        .collect();

    let json = serde_json::to_string(&serializable)
        .map_err(|e| format!("can't serialize to json: {:?}", e))?;

    writer
        .write_all(json.as_bytes())
        .map_err(|e| format!("can't write json to output: {:?}", e))
}
