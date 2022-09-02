pub(crate) mod serde {
    use serde::{Deserialize, Deserializer};

    pub(crate) fn string_as_string_vector<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(vec![s.to_owned()])
    }
}
