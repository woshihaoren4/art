use crate::core::{Ctx, JsonServiceExt, Output, ServiceEntity};
use serde_json::{Value};
use crate::service::custom::Obj;

#[derive(Default, Debug)]
pub struct End {}
#[async_trait::async_trait]
impl JsonServiceExt<Obj, Value> for End {
    async fn output(&self, out: Value) -> anyhow::Result<Output> {
        Ok(Output::value(out))
    }
    async fn call(&self, _ctx: Ctx, input: Obj, _se: ServiceEntity) -> anyhow::Result<Value> {
        Ok(input.into())
    }
}
