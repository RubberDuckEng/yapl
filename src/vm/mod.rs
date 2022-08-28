use serde_json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ParseError,
    SerializeationError,
}

pub fn parse(json: &str) -> Result<serde_json::Value, Error> {
    serde_json::from_str(json).map_err(|_| Error::ParseError)
}

pub fn serialize(value: &serde_json::Value) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|_| Error::SerializeationError)
}
