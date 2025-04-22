use serde_json::{Map, Value};
use wd_tools::PFErr;
use crate::core::{Ctx, JsonInput, JsonServiceExt, Output, ServiceEntity};

#[derive(Debug,Default)]
pub struct Start{
}

#[async_trait::async_trait]
impl JsonServiceExt<Value,Value> for Start {
    async fn input(&self, ctx: Ctx, se: &mut ServiceEntity) -> anyhow::Result<Value> {
        let res = se.transform_config(|c:Option<JsonInput>|{
            c
        });
        let ji = match res {
            None => {
                return anyhow::anyhow!("JsonServiceExt::Start.ServiceEntity config must json Value").err()
            }
            Some(s) => s,
        };
        let input = match ctx.rem_input() {
            Some(s) => {
                if s.downcast_ref::<Value>().is_some() {
                    *(s.downcast::<Value>().unwrap())
                }else{
                    return anyhow::anyhow!("JsonServiceExt::Start.input is not json value").err()
                }
            },
            None=> Value::Null,
        };

        let mut def_val = Value::Object(Map::new());
        ji.transform_value(&mut def_val, input)?;
        Ok(def_val)
    }
    async fn output(&self, out: Value) -> anyhow::Result<Output> {
        Ok(Output::value(out))
    }
    async fn call(&self, _ctx: Ctx, input: Value, _se: ServiceEntity) -> anyhow::Result<Value> {
        Ok(input)
    }
}