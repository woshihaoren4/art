use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use crate::core::{Ctx, Engine, Output, ServiceEntity};

#[async_trait::async_trait]
pub trait EngineSerdeExt: Send {
    fn serde_go<In: Serialize>(ctx:Ctx,input: In)-> anyhow::Result<()>;
    async fn serde_run<In: Serialize + Send,Out:DeserializeOwned>(ctx: Ctx, input: In) -> anyhow::Result<Out>;
}

#[async_trait::async_trait]
impl EngineSerdeExt for Engine{
    fn serde_go<In: Serialize>(ctx: Ctx, input: In) -> anyhow::Result<()> {
        let val= serde_json::to_value(input)?;
        Engine::go(ctx,val);
        Ok(())
    }

    async fn serde_run<In: Serialize+Send, Out: DeserializeOwned>(ctx: Ctx, input: In) -> anyhow::Result<Out> {
        let val= serde_json::to_value(input)?;
        let res = Engine::run::<_,Value>(ctx,val).await?;
        let out = serde_json::from_value::<Out>(res)?;
        Ok(out)
    }
}

#[async_trait::async_trait]
pub trait CtxSerdeExt: Send {
    fn serde_go<In: Serialize>(self,input: In)-> anyhow::Result<()>;
    async fn serde_run<In: Serialize + Send,Out:DeserializeOwned>(self, input: In) -> anyhow::Result<Out>;
}

#[async_trait::async_trait]
impl CtxSerdeExt for Ctx{
    fn serde_go<In: Serialize>(self, input: In) -> anyhow::Result<()> {
        let val= serde_json::to_value(input)?;
        self.go(val);
        Ok(())
    }

    async fn serde_run<In: Serialize+Send, Out: DeserializeOwned>(self, input: In) -> anyhow::Result<Out> {
        let val= serde_json::to_value(input)?;
        let res = self.run::<_,Value>(val).await?;
        let out = serde_json::from_value::<Out>(res)?;
        Ok(out)
    }
}