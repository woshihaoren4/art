mod end;
mod function;
mod start;

pub use end::*;
pub use function::*;
pub use start::*;

#[derive(Debug,serde::Serialize,serde::Deserialize)]
#[serde(untagged)]
pub enum Obj{
    Object(serde_json::Map<String,serde_json::Value>)
}

impl Default for Obj{
    fn default() -> Self {
        Self::Object(serde_json::Map::new())
    }
}

impl From<Obj> for serde_json::Value{
    fn from(value: Obj) -> Self {
        match value {
            Obj::Object(obj) => serde_json::Value::Object(obj)
        }
    }
}