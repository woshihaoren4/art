use crate::core::{Ctx, JsonInput, JsonServiceExt, Output, ServiceEntity};
use serde_json::{Map, Value};

#[derive(Default, Debug)]
pub struct End {}
#[async_trait::async_trait]
impl JsonServiceExt<Value, Value> for End {
    async fn input(&self, ctx: Ctx, se: &mut ServiceEntity) -> anyhow::Result<Value> {
        let res = se.transform_config(|c: Option<JsonInput>| c);
        match res {
            Some(s) => s.transform(ctx, Value::Object(Map::new())).await,
            None => Err(anyhow::anyhow!(
                "End::JsonServiceExt:{}.{} config must json Value",
                se.service_name,
                se.node_name
            )),
        }
    }
    async fn output(&self, out: Value) -> anyhow::Result<Output> {
        Ok(Output::value(out))
    }
    async fn call(&self, _ctx: Ctx, input: Value, _se: ServiceEntity) -> anyhow::Result<Value> {
        Ok(input)
    }
}
