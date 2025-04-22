use serde_json::{Value};
use crate::core::{Ctx, JsonServiceExt, Output, ServiceEntity};

#[derive(Default,Debug)]
pub struct End{}
#[async_trait::async_trait]
impl JsonServiceExt<Value,Value> for End {
    async fn output(&self, out: Value) -> anyhow::Result<Output> {
        Ok(Output::value(out))
    }
    async fn call(&self, _ctx: Ctx, input: Value, _se: ServiceEntity) -> anyhow::Result<Value> {
        Ok(input)
    }
}